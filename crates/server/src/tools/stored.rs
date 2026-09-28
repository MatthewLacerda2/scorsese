//! Running one of the registry's tools on a stored project.
//!
//! Open the row, lay it out ([`lay_out`]), run the tool on the folder exactly
//! as the stdio server would, read the document back, and — if the tool
//! changed it — save it naming the revision it was opened at. A save refused
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
use super::{Caller, Refusal, database, lay_out, project_id};
use crate::projects::{self, ProjectError};

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
    let toolbox = caller.toolbox;
    let attempts = if caller.at.is_some() { 1 } else { ATTEMPTS };
    for _ in 0..attempts {
        let stored = projects::open(&toolbox.pool, caller.user, id)
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
        )
        .await?;
        let root = folder.root().to_path_buf();
        let mut local = arguments.clone();
        local["project"] = Value::String(root.to_string_lossy().into_owned());
        let before = stored.document.to_json().map_err(database)?;

        // The registry's tools are blocking — ffmpeg, the disk — so they run
        // off the server's async threads. Every tool is `Send + Sync`, but a
        // `&dyn` cannot cross into the blocking pool; the registry hands out
        // a fresh one by name.
        let name = tool.name();
        let (outcome, after) = tokio::task::spawn_blocking(move || {
            let tool = super::registered(name).ok_or("the tool went missing")?;
            let outcome = tool.call(&local);
            let after = std::fs::read_to_string(root.join(PROJECT_FILE_NAME)).ok();
            Ok::<_, String>((outcome, after))
        })
        .await
        .map_err(|_| "the tool crashed on the server; that is a bug".to_owned())??;

        let outcome = hide(outcome, folder.root()).map_err(Refusal::Said);
        let Some(after) = after.filter(|after| *after != before) else {
            return outcome;
        };
        let Ok(document) = Project::from_json(&after) else {
            return outcome;
        };
        match projects::save(
            &toolbox.pool,
            caller.user,
            id,
            stored.summary.revision,
            &document,
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
                 you. The picture is in the reply; render is how a file is made to download."
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
