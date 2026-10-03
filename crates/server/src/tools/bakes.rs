//! A synthesis bake made on the server, kept in its owner's library (#560).
//!
//! `synth_bake` writes its WAV into the folder's `generated/` and points the
//! asset at it by path and `sha256` — exactly what it does in a `.scor` folder.
//! But the folder is gone the moment the tool answers, and a stored document
//! may only name files its owner's library holds (`projects::media`). So every
//! bake the call produced is **admitted to the library** before the document
//! is saved: a generated item like a Veo shot, carrying its address — the
//! digest of recipe and synthesiser that names the file — as its brief hash.
//! From then on it is linked into every layout by hash like any other file,
//! and a render reads it there.
//!
//! Only a **regular file** the call wrote is a new bake. A bake that was
//! already in the library was linked into the folder, and a link is never
//! admitted twice; one somebody else's hash names was never linked at all.

use std::path::Path;

use scorsese_core::{GENERATED_DIR, Project};

use crate::db::UserId;
use crate::library::{Arrival, Kind, Library, LibraryError};
use crate::projects::media::hashes;

/// Admit to `user`'s library every bake in the folder at `root` that `after`
/// names and `before` did not.
pub(super) async fn keep(
    library: &Library,
    user: UserId,
    before: &Project,
    after: &Project,
    root: &Path,
) -> Result<(), String> {
    let known = hashes(before);
    for asset in after
        .assets
        .iter()
        .filter(|asset| asset.kind.is_synthesized())
    {
        let (Some(path), Some(sha256)) = (&asset.path, &asset.sha256) else {
            continue;
        };
        let Some(address) = path
            .as_str()
            .strip_prefix(&format!("{GENERATED_DIR}/"))
            .and_then(|name| name.strip_suffix(".wav"))
        else {
            continue;
        };
        let file = path.resolve(root);
        let written = file.symlink_metadata().is_ok_and(|meta| meta.is_file());
        if !written || known.contains(sha256.as_str()) {
            continue;
        }
        let arrival = Arrival {
            file,
            name: format!("{}.wav", asset.id),
            kind: Kind::Audio,
            extension: "wav".to_owned(),
            announced: Some(sha256.clone()),
            brief_hash: Some(address.to_owned()),
        };
        library
            .keep_generated(user, address, arrival)
            .await
            .map_err(|error| refused(&asset.id.to_string(), error))?;
    }
    Ok(())
}

/// A bake the library would not take, said to the caller.
fn refused(asset: &str, error: LibraryError) -> String {
    match error {
        LibraryError::Rejected(why) | LibraryError::Invalid(why) => {
            format!("the bake of `{asset}` could not be kept in your library: {why}")
        }
        other => super::database(other),
    }
}
