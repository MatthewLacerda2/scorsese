//! Bringing a web page into the project's `pages/`.
//!
//! A page arrives the way any file does — copied in, never linked, so the
//! project still survives `scp -r` — but it is not media, and two things that
//! import does to media do not apply to it:
//!
//! - **No probe.** ffprobe has nothing to say about a document; there is no
//!   frame rate or duration in it, so there is no `media` block to fill in.
//! - **No hash.** A page is authored, and the point of authoring is editing.
//!   A recorded hash would turn every edit into a "changed since import"
//!   fault, the way a recipe — the other authored document — would be if it
//!   carried one. So a page is never deduplicated by content either: importing
//!   the same file twice is two pages, which is what two documents that are
//!   about to diverge are.

use std::fs;
use std::path::Path;

use crate::asset::{Asset, AssetId, AssetKind, is_page_path};
use crate::path::ProjectPath;
use crate::project::{PAGES_DIR, Project};

use super::import::ImportError;
use super::naming::{unique_asset_id, unique_file_name};

/// Copies a page into `pages/` and adds it to the assets table as an `html`
/// asset. Returns the id to reference from a clip.
pub(super) fn import_page(
    project: &mut Project,
    project_root: &Path,
    source: &Path,
) -> Result<AssetId, ImportError> {
    // Refused before anything is copied: validation would refuse the asset
    // anyway, and a page that cannot be one should not leave a file behind.
    if !source.to_str().is_some_and(is_page_path) {
        return Err(ImportError::KindMismatch {
            path: source.to_path_buf(),
            kind: AssetKind::Html,
            found: "no .html extension",
        });
    }
    let bytes = fs::read(source).map_err(|error| ImportError::Unreadable {
        path: source.to_path_buf(),
        source: error,
    })?;
    let pages_dir = project_root.join(PAGES_DIR);
    fs::create_dir_all(&pages_dir).map_err(|error| ImportError::Unwritable {
        path: pages_dir.clone(),
        source: error,
    })?;
    let file_name = unique_file_name(&pages_dir, source);
    let destination = pages_dir.join(&file_name);
    fs::write(&destination, bytes).map_err(|error| ImportError::Unwritable {
        path: destination.clone(),
        source: error,
    })?;

    let id = unique_asset_id(project, &file_name);
    let path = ProjectPath::new(format!("{PAGES_DIR}/{file_name}"));
    project
        .assets
        .push(Asset::imported(id.clone(), AssetKind::Html, path));
    Ok(id)
}
