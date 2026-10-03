//! Running one of the registry's tools on a stored project.
//!
//! Open the row and its kept files, lay it out ([`lay_out`]), run the tool on
//! the folder exactly as the stdio server would, read the document and the
//! kept files back (`projects::files::gather`), and — if the tool changed
//! either — save both naming the revision it was opened at, after keeping any
//! bake it made in the library (`bakes`). A save refused
//! because somebody else wrote in between (the browser, the assistant, a
//! second client) is not an error to hand back: the tool is a pure function
//! of the document, so it is run again on what is there now, a few times,
//! before the caller is told the project would not hold still.
//!
//! **Except for an edit made against a revision** — the web editor's
//! ([`Toolbox::edit`](super::Toolbox::edit)). A drag is worked out on the
//! timeline the user was looking at; if the project has moved since, running
//! it again on what is there now would land an edit nobody saw the result of.
//! So that call runs once, and a project at any other revision — when it is
//! opened or when it is saved — is [`Refusal::Moved`], the conflict rule the
//! projects API keeps (#534).
//!
//! The folder's own path never reaches the caller. It is where this machine
//! happened to put a copy for a moment — meaningless to a client, and a
//! detail of the server nobody outside it needs.

use scorsese_core::{PROJECT_FILE_NAME, Project, ProjectPath};
use scorsese_mcp::{Part, Reply, Tool};
use serde_json::Value;

use super::surface::Serve;
use super::{Caller, Refusal, bakes, carried, database, lay_out, project_id};
use crate::projects::{self, ProjectError, files};

/// How many times a call is run again on a project that moved under it.
const ATTEMPTS: usize = 3;

/// Run `tool` for `caller` on the project `arguments` names.
pub(super) async fn run(
    caller: &Caller<'_>,
    tool: &dyn Tool,
    serve: Serve,
    arguments: &Value,
) -> Result<Reply, Refusal> {
    let id = project_id(arguments)?;
    held(serve, arguments)?;
    let item = carried::fetch(caller, serve, arguments).await?;
    let toolbox = caller.toolbox;
    let attempts = if caller.at.is_some() { 1 } else { ATTEMPTS };
    for _ in 0..attempts {
        if caller.cancel.is_cancelled() {
            return Err("cancelled before it ran, so nothing was changed"
                .to_owned()
                .into());
        }
        let (stored, kept) = projects::open_with_files(&toolbox.pool, caller.user, id)
            .await
            .map_err(opened)?;
        if caller.at.is_some_and(|at| at != stored.summary.revision) {
            return Err(Refusal::Moved);
        }
        let folder = lay_out(
            &toolbox.pool,
            &toolbox.storage,
            caller.user,
            &stored.document,
            &kept,
        )
        .await?;
        let root = folder.root().to_path_buf();
        let mut local = arguments.clone();
        local["project"] = Value::String(root.to_string_lossy().into_owned());
        carried::bring(caller, serve, item.as_ref(), &mut local, &root)?;
        let before = stored.document.to_json().map_err(database)?;

        // The registry's tools are blocking — ffmpeg, the disk — so they run
        // off the server's async threads. Every tool is `Send + Sync`, but a
        // `&dyn` cannot cross into the blocking pool; the registry hands out
        // a fresh one by name.
        let (name, cancel) = (tool.name(), caller.cancel.clone());
        let (opened_with, laid) = (stored.document.clone(), kept.clone());
        let (outcome, after, gathered) = tokio::task::spawn_blocking(move || {
            let tool = super::registered(name).ok_or("the tool went missing")?;
            let outcome = tool.call_cancellable(&local, &cancel);
            let after = std::fs::read_to_string(root.join(PROJECT_FILE_NAME))
                .ok()
                .and_then(|after| Project::from_json(&after).ok().map(|read| (after, read)));
            let reading = after.as_ref().map_or(&opened_with, |(_, read)| read);
            let gathered = files::gather(&root, reading, &laid);
            Ok::<_, String>((outcome, after, gathered))
        })
        .await
        .map_err(|_| "the tool crashed on the server; that is a bug".to_owned())??;

        let outcome = carried::keep(caller, serve, hide(outcome, folder.root()), folder.root())
            .await
            .map_err(Refusal::Said);
        let gathered = gathered.map_err(|why| format!("{why} — nothing was saved"))?;
        let document = after
            .filter(|(after, _)| *after != before)
            .map(|(_, read)| read);
        if document.is_none() && gathered == kept {
            return outcome;
        }
        let document = document.unwrap_or_else(|| stored.document.clone());
        bakes::keep(
            &toolbox.library,
            caller.user,
            &stored.document,
            &document,
            folder.root(),
        )
        .await?;
        match projects::save_with_files(
            &toolbox.pool,
            caller.user,
            id,
            stored.summary.revision,
            &document,
            &gathered,
        )
        .await
        {
            Ok(_) => return outcome,
            Err(ProjectError::Conflict { .. }) if caller.at.is_some() => {
                return Err(Refusal::Moved);
            }
            Err(ProjectError::Conflict { .. }) => {}
            Err(refused @ ProjectError::UnknownFiles { .. }) => {
                return Err(refused.to_string().into());
            }
            Err(error) => return Err(database(error).into()),
        }
    }
    Err(
        "the project kept changing while this ran, so nothing was saved; call again"
            .to_owned()
            .into(),
    )
}

/// Refuse the arguments the web holds back: a file argument that is not a
/// path inside the project, or one the web does not take at all.
fn held(serve: Serve, arguments: &Value) -> Result<(), String> {
    match serve {
        Serve::Confined(fields) => {
            for field in fields {
                let Some(given) = arguments.get(*field).and_then(Value::as_str) else {
                    continue;
                };
                ProjectPath::new(given).check().map_err(|problem| {
                    format!(
                        "{field}: {given} is not a path inside the project ({problem}) — on \
                         the hosted server only the project's own files can be read"
                    )
                })?;
            }
            Ok(())
        }
        Serve::Without(fields) => {
            match fields.iter().find(|field| arguments.get(**field).is_some()) {
                Some(field) => Err(format!(
                    "{field} is not taken on the hosted server: nothing is kept on its disk for \
                 you. What the tool found is in its reply; render is how a file is made to \
                 download."
                )),
                None => Ok(()),
            }
        }
        Serve::IntoLibrary { without, .. } => {
            match without
                .iter()
                .find(|field| arguments.get(**field).is_some())
            {
                Some(field) => Err(format!(
                    "{field} is not taken on the hosted server: the file is kept in your \
                     library, where you download it — leave {field} out"
                )),
                None => Ok(()),
            }
        }
        _ => Ok(()),
    }
}

/// The reply, with the folder's path taken out of every word of it.
fn hide(outcome: Result<Reply, String>, root: &std::path::Path) -> Result<Reply, String> {
    let root = root.to_string_lossy().into_owned();
    let clean = |text: String| {
        text.replace(&format!("{root}/"), "")
            .replace(&root, "the project")
    };
    match outcome {
        Ok(reply) => Ok(Reply {
            parts: reply
                .parts
                .into_iter()
                .map(|part| Part {
                    text: clean(part.text),
                    image: part.image,
                })
                .collect(),
        }),
        Err(refusal) => Err(clean(refusal)),
    }
}

/// A project that could not be opened, said to the caller.
fn opened(error: ProjectError) -> String {
    match error {
        ProjectError::NotFound => "there is no such project of yours".to_owned(),
        other => database(other),
    }
}
