//! Collecting assets no clip refers to.

use std::fs;
use std::path::{Path, PathBuf};

use crate::asset::{Asset, AssetId, AssetKind};
use crate::project::Project;
use crate::words::Words;

/// Assets that no clip references, in table order.
///
/// Listing is separate from removing on purpose: deleting media is not
/// undoable, so the caller decides after seeing the list.
///
/// A still an image sequence plays is in use whether or not a clip shows the
/// sequence: the sequence names it, and taking it away would leave the
/// sequence naming nothing. So a sequence nobody places is collected first,
/// and its stills by the collection after.
pub fn unused_assets(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| !in_use(project, &asset.id))
        .map(|asset| asset.id.clone())
        .collect()
}

/// Whether a clip shows this asset or a sequence plays it.
fn in_use(project: &Project, id: &AssetId) -> bool {
    project.every_clip().any(|(_, clip)| &clip.asset == id)
        || project
            .assets
            .iter()
            .filter_map(|asset| asset.sequence.as_ref())
            .any(|sequence| sequence.stills.contains(id))
}

/// Drops these assets from the table and deletes the media files they own —
/// never a page, which is a document somebody wrote. A generated line's word
/// timings, kept beside its audio, go with it.
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
        if in_use(project, id) {
            return Err(GcError::StillReferenced { id: id.clone() });
        }
        let Some(index) = project.assets.iter().position(|asset| &asset.id == id) else {
            return Err(GcError::NoSuchAsset { id: id.clone() });
        };
        let asset = project.assets.remove(index);

        // A page is authored, not imported: dropping its row is collecting an
        // asset, deleting its file would be losing work. It stays in `pages/`,
        // the way a recipe stays in `recipes/`.
        if let Some(path) = asset.path.as_ref().filter(|_| asset.kind.is_media()) {
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
        report.bytes_freed += remove_timings(&asset, project_root)?;
        report.removed.push(asset.id);
    }
    Ok(report)
}

/// Deletes a generated line's word timings, which live beside its audio and
/// mean nothing once the line is gone, returning the bytes they held. A line
/// generated without timings, or whose timings are already gone, frees none.
fn remove_timings(asset: &Asset, project_root: &Path) -> Result<u64, GcError> {
    let Some(audio) = asset.path.as_ref() else {
        return Ok(0);
    };
    if asset.kind != AssetKind::GeneratedAudio {
        return Ok(0);
    }
    let file = Words::beside(audio).resolve(project_root);
    let Ok(metadata) = fs::metadata(&file) else {
        return Ok(0);
    };
    fs::remove_file(&file).map_err(|source| GcError::Undeletable {
        path: file.clone(),
        source,
    })?;
    Ok(metadata.len())
}

/// What a collection actually did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GcReport {
    /// The assets dropped from the table, in the order they were asked for.
    pub removed: Vec<AssetId>,
    /// Media files actually unlinked. Fewer than `removed` when some entries
    /// were inline or already gone. A generated line's word timings go with
    /// its audio and are not counted apart from it.
    pub files_deleted: usize,
    /// Entries whose file was already absent.
    pub files_missing: usize,
    /// How much disk the deleted files were holding, timings included — the
    /// number worth showing a human deciding whether to collect.
    pub bytes_freed: u64,
}

/// Why a collection stopped.
#[derive(Debug, thiserror::Error)]
pub enum GcError {
    /// Collecting it would leave a clip, or a sequence, pointing at nothing,
    /// which is exactly what validation exists to prevent.
    #[error("asset `{id}` is still used by a clip or an image sequence")]
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
