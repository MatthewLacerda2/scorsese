//! Media a tool downloaded into the project, kept in its owner's library
//! (#900).
//!
//! `stock_import` downloads a Pixabay file into the folder's `assets/` and
//! adds an ordinary `video` or `image` asset naming it by path and `sha256` —
//! exactly what it does in a `.scor` folder. The folder is gone the moment the
//! tool answers, and a stored document may only name files its owner's
//! library holds, so every such file is **admitted to the library** before
//! the document is saved, as an upload would be. From then on it is linked
//! into every layout by hash like any other file.
//!
//! Only a **regular file** a new asset names counts. Every file the project
//! already had was linked into the folder, and a link is never admitted.

use std::path::Path;

use scorsese_core::{AssetKind, Project};

use crate::db::UserId;
use crate::library::{Arrival, Kind, Library, LibraryError};
use crate::projects::media::hashes;

/// Admit to `user`'s library every media file in the folder at `root` that
/// `after` names and `before` did not.
pub async fn keep(
    library: &Library,
    user: UserId,
    before: &Project,
    after: &Project,
    root: &Path,
) -> Result<(), String> {
    let known = hashes(before);
    for asset in &after.assets {
        let kind = match asset.kind {
            AssetKind::Video => Kind::Video,
            AssetKind::Image => Kind::Image,
            _ => continue,
        };
        let (Some(path), Some(sha256)) = (&asset.path, &asset.sha256) else {
            continue;
        };
        let file = path.resolve(root);
        let written = file.symlink_metadata().is_ok_and(|meta| meta.is_file());
        if !written || known.contains(sha256.as_str()) {
            continue;
        }
        let name = file.file_name().map_or_else(
            || asset.id.to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let extension = file
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let arrival = Arrival {
            file,
            name,
            kind,
            extension,
            announced: Some(sha256.clone()),
            brief_hash: None,
        };
        match library.admit(user, arrival).await {
            // The same bytes already in the library are the file the
            // document needs, under whatever name the user gave them.
            Ok(_) | Err(LibraryError::Duplicate { .. }) => {}
            Err(LibraryError::Rejected(why) | LibraryError::Invalid(why)) => {
                return Err(format!(
                    "`{}` could not be kept in your library: {why}",
                    asset.id
                ));
            }
            Err(other) => return Err(super::database(other)),
        }
    }
    Ok(())
}
