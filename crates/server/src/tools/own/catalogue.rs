//! `library` and `import`: what files a user has, and bringing some into a
//! project.
//!
//! Locally `import` copies a file from anywhere on the machine into the
//! project's `assets/`. On the server, "anywhere on the machine" is somebody
//! else's business, and every file a user can use is already in their library
//! — uploaded through the web app, or generated. So importing is naming
//! library items: each becomes an asset at `assets/<sha256>.<ext>`
//! (`projects::media::library_path`) through `scorsese_core::pool::
//! reference_asset`, the document half of an import, with the same id rules
//! and the same answer for a file the project already has. A page is the one
//! file copied rather than referred to ([`super::pages`]). A document is the
//! one never imported: it is for reading ([`super::document`]), not placing.

use scorsese_core::bytes::format_bytes;
use scorsese_core::pool::{Reference, reference_asset};
use scorsese_core::{AssetKind, Project};
use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::super::surface::project_property;
use super::super::{Caller, database, project_id};
use super::pages::bring_page;
use crate::library::{Filter, Item, Kind, LibraryError};
use crate::projects::{self, ProjectError, ProjectFiles, media::library_path};

/// How many times an import is made again on a project that moved under it.
const ATTEMPTS: usize = 3;

/// How a client names the listing.
pub(super) const LIBRARY: &str = "library";

/// What the listing does.
pub(super) const LIBRARY_SAYS: &str = "List the files in your library — every video, picture, \
sound and web page you uploaded or generated, and the text documents (kind text) you uploaded — \
with the id import takes, newest first. Each line is the id, the name, the kind, what probing found (length, size) and the \
description, when it has one: read the descriptions to choose a file. Narrow it by kind, by a \
word in the name, or to the files one project already uses. A generated file says so. A \
document is a script, shot list or briefing the person wrote: read it with library_read; it \
is never imported.";

/// `library`'s arguments.
pub(super) fn library_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["video", "image", "audio", "html", "text"],
                "description": "Only files of this kind."
            },
            "search": {
                "type": "string",
                "description": "Only files whose name contains this, ignoring case."
            },
            "project": {
                "type": "integer",
                "description": "Only the files this project already uses, by its id."
            }
        }
    })
}

/// How a client names the import.
pub(super) const IMPORT: &str = "import";

/// What the import does.
pub(super) const IMPORT_SAYS: &str = "Bring files from your library into a project's assets \
table, ready for a clip to reference — the hosted server's import. Name them by the ids \
`library` lists. Media is not copied: the project refers to the library's file, so a file \
used in ten projects is stored once. A web page is copied into the project's pages/, as an \
html asset page_write and page_read work on, so editing it there leaves the library's copy and \
every other project alone. A file the project already has is not added twice; its existing \
asset id is the answer. The reply names the asset id each file got, which is what \
place_clip takes. New files reach the library by uploading them in the web app.";

/// `import`'s arguments.
pub(super) fn import_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": project_property(),
            "items": {
                "type": "array",
                "items": { "type": "integer" },
                "minItems": 1,
                "description": "The library files to bring in, by the ids `library` lists."
            }
        },
        "required": ["project", "items"]
    })
}

/// The caller's library, as lines to choose from.
pub(super) async fn library(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let filter: Filter = serde_json::from_value(arguments.clone())
        .map_err(|error| format!("the arguments do not read: {error}"))?;
    let items = caller
        .toolbox
        .library
        .catalogue(caller.user, &filter)
        .await
        .map_err(said)?;
    if items.is_empty() {
        return Ok(
            "No files match. Files reach your library by uploading them in the web \
                   app, or by generating them."
                .into(),
        );
    }
    Ok(items.iter().map(line).collect::<Vec<_>>().join("\n").into())
}

/// One item as a line.
fn line(item: &Item) -> String {
    let mut facts = vec![item.kind.as_str().to_owned()];
    if let Some(seconds) = item.media.duration_seconds {
        facts.push(format!("{seconds:.1}s"));
    }
    if let (Some(width), Some(height)) = (item.media.width, item.media.height) {
        facts.push(format!("{width}x{height}"));
    }
    if item.kind == Kind::Text {
        facts.push(format_bytes(item.size_bytes.unsigned_abs()));
    }
    if item.brief_hash.is_some() {
        facts.push("generated".to_owned());
    }
    let described = item
        .description
        .as_deref()
        .map_or(String::new(), |words| format!(" — {words}"));
    format!(
        "{} — {} ({}){described}",
        item.id,
        item.name,
        facts.join(", ")
    )
}

/// Library items into a project.
pub(super) async fn import(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let project = project_id(arguments)?;
    let ids: Vec<i64> = arguments
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_i64).collect())
        .filter(|ids: &Vec<i64>| !ids.is_empty())
        .ok_or("`items` is required: the library ids to bring in, as `library` lists them")?;
    let library = &caller.toolbox.library;
    let mut files = Vec::with_capacity(ids.len());
    for id in &ids {
        let item = library
            .get(caller.user, *id)
            .await
            .map_err(|error| match error {
                LibraryError::NotFound => format!("there is no file {id} in your library"),
                other => said(other),
            })?;
        let page = match item.kind {
            Kind::Text => return Err(unimportable(&item)),
            Kind::Html => {
                let file =
                    library
                        .storage()
                        .library_file(caller.user, &item.sha256, &item.extension);
                Some(tokio::fs::read_to_string(file).await.map_err(database)?)
            }
            _ => None,
        };
        files.push((item, page));
    }
    let pool = &caller.toolbox.pool;
    let opened = |error| match error {
        ProjectError::NotFound => "there is no such project of yours".to_owned(),
        other => database(other),
    };
    for _ in 0..ATTEMPTS {
        let (stored, mut kept) = projects::open_with_files(pool, caller.user, project)
            .await
            .map_err(opened)?;
        let mut document = stored.document;
        let lines = files
            .iter()
            .map(|(item, page)| bring(&mut document, &mut kept, item, page.as_deref()))
            .collect::<Result<Vec<_>, _>>()?;
        let revision = stored.summary.revision;
        let saved = if files.iter().any(|(_, page)| page.is_some()) {
            projects::save_with_files(pool, caller.user, project, revision, &document, &kept).await
        } else {
            projects::save(pool, caller.user, project, revision, &document).await
        };
        match saved {
            Ok(_) => return Ok(lines.join("\n").into()),
            Err(ProjectError::Conflict { .. }) => {}
            Err(refused @ ProjectError::UnknownFiles { .. }) => return Err(refused.to_string()),
            Err(error) => return Err(opened(error)),
        }
    }
    Err("the project kept changing while this ran, so nothing was imported; call again".into())
}

/// Bring one library item into `document` — a page's `text` into `kept` too
/// — as a line saying which asset it is.
fn bring(
    document: &mut Project,
    kept: &mut ProjectFiles,
    item: &Item,
    text: Option<&str>,
) -> Result<String, String> {
    let (asset, added) = match text {
        Some(text) => bring_page(document, kept, &item.name, text.to_owned())?,
        None => {
            let known = document
                .assets
                .iter()
                .any(|asset| asset.sha256.as_deref() == Some(item.sha256.as_str()));
            let kind = item.kind.asset_kind().ok_or_else(|| unimportable(item))?;
            (reference_asset(document, reference(item, kind)), !known)
        }
    };
    let how = if added {
        "added"
    } else {
        "already in the project"
    };
    Ok(format!(
        "{} “{}” → asset `{asset}` ({how})",
        item.id, item.name
    ))
}

/// Why a document is not imported: it is read, never placed.
fn unimportable(item: &Item) -> String {
    format!(
        "{} “{}” is a document, which is never imported into a project: read it with \
         library_read",
        item.id, item.name
    )
}

/// A library item as the asset of `kind` a project refers to it by.
fn reference(item: &Item, kind: AssetKind) -> Reference {
    Reference {
        name: item.name.clone(),
        kind,
        path: library_path(&item.sha256, Some(&item.extension)),
        sha256: item.sha256.clone(),
        media: item.media,
    }
}

/// A library failure, said to the caller.
fn said(error: LibraryError) -> String {
    match error {
        LibraryError::Invalid(why) => why,
        other => database(other),
    }
}
