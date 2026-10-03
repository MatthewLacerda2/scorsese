//! Files a tool takes from the caller's library, or leaves in it (#678).
//!
//! A registry tool reads and writes paths. On the web the only files a call
//! can name are the project's own, laid out for its length (`folder`), and
//! anything written outside what the project keeps is gone the moment the
//! tool answers. That is right for every file a project is made of; it is
//! wrong for one that is neither media nor a kept recipe — a `.mid`. So the
//! library holds it, like any other file of the user's:
//!
//! - **In** ([`Serve::FromLibrary`]): the caller names a library item by id
//!   where the registry takes a path; the item is linked into the folder under
//!   `cache/`, and the tool is handed that path. Its name, without the
//!   extension, is what the tool would have taken from the path's — so an
//!   import names its asset after what the user called the file, not after
//!   its hash.
//! - **Out** ([`Serve::IntoLibrary`]): every regular file the tool wrote under
//!   its default folder is admitted to the library — checked exactly as an
//!   upload is — and the reply says where it went. The same bytes written
//!   again are the item already there, not a refusal: the caller asked for an
//!   export and has one.

use std::path::Path;

use scorsese_core::CACHE_DIR;
use scorsese_mcp::{Part, Reply};
use serde_json::Value;

use super::surface::Serve;
use super::{Caller, database, folder};
use crate::library::{Arrival, Item, LibraryError};

/// The library item a [`Serve::FromLibrary`] call names, checked to be the
/// caller's and of the kind the tool reads — `None` for any other call.
pub(super) async fn fetch(
    caller: &Caller<'_>,
    serve: Serve,
    arguments: &Value,
) -> Result<Option<Item>, String> {
    let Serve::FromLibrary { kind, .. } = serve else {
        return Ok(None);
    };
    let given = arguments.get("item");
    let id = given
        .and_then(Value::as_i64)
        .or_else(|| given.and_then(Value::as_str)?.trim().parse().ok())
        .ok_or_else(|| {
            format!(
                "`item` is required: the {} file to read, by the id `library` lists",
                kind.label()
            )
        })?;
    let item = caller
        .toolbox
        .library
        .get(caller.user, id)
        .await
        .map_err(|error| match error {
            LibraryError::NotFound => format!("there is no file {id} in your library"),
            other => database(other),
        })?;
    if item.kind != kind {
        return Err(format!(
            "{id} “{}” is a {} file, not a {} one",
            item.name,
            item.kind.label(),
            kind.label()
        ));
    }
    Ok(Some(item))
}

/// Link `item` into the folder at `root` and hand the tool its path, in
/// `local`, the arguments the tool is run with.
pub(super) fn bring(
    caller: &Caller<'_>,
    serve: Serve,
    item: Option<&Item>,
    local: &mut Value,
    root: &Path,
) -> Result<(), String> {
    let (Serve::FromLibrary { field, .. }, Some(item)) = (serve, item) else {
        return Ok(());
    };
    let storage = &caller.toolbox.storage;
    let source = storage.library_file(caller.user, &item.sha256, &item.extension);
    let relative = format!("{CACHE_DIR}/library/{}.{}", item.sha256, item.extension);
    folder::link(&source, &root.join(&relative))?;
    if let Some(arguments) = local.as_object_mut() {
        arguments.remove("item");
        arguments.insert(field.to_owned(), Value::String(relative));
        if !arguments.contains_key("name") {
            let stem = Path::new(&item.name)
                .file_stem()
                .map_or_else(|| item.name.clone(), |stem| stem.to_string_lossy().into());
            arguments.insert("name".to_owned(), Value::String(stem));
        }
    }
    Ok(())
}

/// Keep every file a [`Serve::IntoLibrary`] tool wrote under the folder at
/// `root` in the caller's library, and say so in its reply. Any other call's
/// outcome, and a refusal, pass through untouched.
pub(super) async fn keep(
    caller: &Caller<'_>,
    serve: Serve,
    outcome: Result<Reply, String>,
    root: &Path,
) -> Result<Reply, String> {
    let Serve::IntoLibrary { dir, kind, .. } = serve else {
        return outcome;
    };
    let mut reply = outcome?;
    let mut written: Vec<_> = std::fs::read_dir(root.join(dir))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.symlink_metadata().is_ok_and(|meta| meta.is_file()))
        .collect();
    written.sort();
    for file in written {
        let name = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let extension = file
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let arrival = Arrival {
            file,
            name: name.clone(),
            kind,
            extension,
            announced: None,
            brief_hash: None,
        };
        let said = match caller.toolbox.library.admit(caller.user, arrival).await {
            Ok(item) => format!(
                "Kept in your library as “{}” (file {}), where you can download it.",
                item.name, item.id
            ),
            Err(LibraryError::Duplicate { id, name }) => format!(
                "Your library already has exactly this file, as “{name}” (file {id}); it was \
                 not kept twice."
            ),
            Err(LibraryError::Rejected(why) | LibraryError::Invalid(why)) => {
                return Err(format!("{name} could not be kept in your library: {why}"));
            }
            Err(other) => return Err(database(other)),
        };
        reply.parts.push(Part {
            text: said,
            image: None,
        });
    }
    Ok(reply)
}
