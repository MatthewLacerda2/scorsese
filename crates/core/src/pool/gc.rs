//! Collecting assets no clip refers to.

use std::fs;
use std::path::{Path, PathBuf};

use crate::asset::AssetId;
use crate::project::Project;

/// Assets that no clip references, in table order.
///
/// Listing is separate from removing on purpose: deleting media is not
/// undoable, so the caller decides after seeing the list.
pub fn unused_assets(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| !project.clips().any(|(_, clip)| clip.asset == asset.id))
        .map(|asset| asset.id.clone())
        .collect()
}

/// Drops these assets from the table and deletes the files they own.
///
/// An asset still referenced by a clip is refused rather than removed — that
/// would leave a dangling reference, which is precisely what validation
/// exists to prevent.
pub fn remove_assets(
    project: &mut Project,
    project_root: &Path,
    ids: &[AssetId],
) -> Result<GcReport, GcError> {
    let mut report = GcReport::default();

    for id in ids {
        if project.clips().any(|(_, clip)| &clip.asset == id) {
            return Err(GcError::StillReferenced { id: id.clone() });
        }
        let Some(index) = project.assets.iter().position(|asset| &asset.id == id) else {
            return Err(GcError::NoSuchAsset { id: id.clone() });
        };
        let asset = project.assets.remove(index);

        if let Some(path) = &asset.path {
            let file = path.resolve(project_root);
            match fs::metadata(&file) {
                Ok(metadata) => {
                    fs::remove_file(&file).map_err(|source| GcError::Undeletable {
                        path: file.clone(),
                        source,
                    })?;
                    report.bytes_freed += metadata.len();
                    report.files_deleted += 1;
                }
                // Already gone: collecting it is still the right outcome.
                Err(_) => report.files_missing += 1,
            }
        }
        report.removed.push(asset.id);
    }
    Ok(report)
}

/// What a collection actually did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GcReport {
    /// The assets dropped from the table, in the order they were asked for.
    pub removed: Vec<AssetId>,
    /// Files actually unlinked. Fewer than `removed` when some entries were
    /// inline or already gone.
    pub files_deleted: usize,
    /// Entries whose file was already absent.
    pub files_missing: usize,
    /// How much disk the deleted files were holding — the number worth
    /// showing a human deciding whether to collect.
    pub bytes_freed: u64,
}

/// Why a collection stopped.
#[derive(Debug, thiserror::Error)]
pub enum GcError {
    /// Collecting it would leave a clip pointing at nothing, which is exactly
    /// what validation exists to prevent.
    #[error("asset `{id}` is still used by a clip")]
    StillReferenced {
        /// The asset a clip still refers to.
        id: AssetId,
    },
    /// An id that is not in the assets table — a stale list, or a typo.
    #[error("no asset `{id}` in this project")]
    NoSuchAsset {
        /// The id that matched nothing.
        id: AssetId,
    },
    /// The entry is gone from the table, but its file could not be unlinked.
    /// Collection stops here rather than carrying on half-done.
    #[error("cannot delete {}: {source}", path.display())]
    Undeletable {
        /// The file that survived.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
}
