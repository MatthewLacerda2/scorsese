//! Collecting assets nothing refers to.

mod pages;

use std::fs;
use std::path::{Path, PathBuf};

use crate::asset::{Asset, AssetId};
use crate::project::Project;
use crate::validate::ValidationErrors;
use crate::words::Words;

pub use pages::{NamedByPage, named_by_pages};

/// Assets that no clip shows and no other asset names, in table order.
///
/// Listing is separate from removing on purpose: deleting media is not
/// undoable, so the caller decides after seeing the list.
///
/// An asset another one names is in use whether or not a clip shows the one
/// naming it — a still a sequence plays, a picture a shot starts from or a
/// generated still is drawn after ([`Asset::references`]). Taking it away
/// would leave that asset naming nothing. So a sequence nobody places is
/// collected first, and its stills by the collection after.
///
/// What a page's *text* names is not in the document, so it is not here:
/// [`named_by_pages`] reads the pages, and [`remove_assets`] refuses those too.
///
/// Nor is whether an asset was paid for: an unused Veo shot is listed like
/// any other, and it is [`remove_assets`] that keeps it unless told
/// [`Paid::Delete`].
pub fn unused_assets(project: &Project) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| !in_use(project, &asset.id))
        .map(|asset| asset.id.clone())
        .collect()
}

/// Whether a clip shows this asset or another asset names it.
fn in_use(project: &Project, id: &AssetId) -> bool {
    project.clips_of(id).next().is_some()
        || project
            .assets
            .iter()
            .any(|asset| asset.references().any(|named| named == id))
}

/// Drops these assets from the table and deletes the media files they own —
/// never a page, which is a document somebody wrote. A generated line's word
/// timings, kept beside its audio, go with it, and only when the line does; so
/// does an imported recording's transcript.
///
/// Every id is checked, and the project without them validated, **before any
/// file is touched**: an asset still shown, named by another or named by a
/// page's text is refused, and so is a removal that would leave a project
/// [`crate::Project::load`] refuses. Deleting media is not undoable, so a
/// collection that cannot finish never starts. The ids are taken in order, so
/// a sequence followed by its stills is one collection.
///
/// **A paid asset ([`Asset::is_paid`]) is refused under [`Paid::Keep`]**:
/// what was bought from a provider is never deleted unless the user asked for
/// exactly that. Even under [`Paid::Delete`] only the file the asset's row
/// points at goes; the older generations a regenerated brief left beside it
/// (`<asset>-<brief hash>`) belong to no asset, and collection never touches
/// them.
pub fn remove_assets(
    project: &mut Project,
    project_root: &Path,
    ids: &[AssetId],
    paid: Paid,
) -> Result<GcReport, GcError> {
    let named = named_by_pages(project, project_root);
    let mut after = project.clone();
    for id in ids {
        if in_use(&after, id) {
            return Err(GcError::StillReferenced { id: id.clone() });
        }
        if let Some(by) = named.iter().find(|named| &named.asset == id) {
            return Err(GcError::NamedByPage {
                id: id.clone(),
                page: by.page.clone(),
            });
        }
        let Some(index) = after.assets.iter().position(|asset| &asset.id == id) else {
            return Err(GcError::NoSuchAsset { id: id.clone() });
        };
        if paid == Paid::Keep && after.assets[index].is_paid() {
            return Err(GcError::Paid { id: id.clone() });
        }
        after.assets.remove(index);
    }
    after.validate()?;

    let mut report = GcReport::default();
    for id in ids {
        let Some(index) = project.assets.iter().position(|asset| &asset.id == id) else {
            continue;
        };
        let asset = project.assets.remove(index);

        // A page is authored, not imported: dropping its row is collecting an
        // asset, deleting its file would be losing work. It stays in `pages/`,
        // the way a recipe stays in `recipes/`. A path out of the project
        // names nothing of the project's to delete.
        let file = asset
            .path
            .as_ref()
            .filter(|_| asset.kind.is_media())
            .and_then(|path| path.resolve(project_root).ok());
        if let Some(file) = file {
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

/// Deletes a generated line's word timings, or an imported recording's
/// transcript, which live beside its file and mean nothing once it is gone,
/// returning the bytes they held. One without them, or whose timings are
/// already gone, frees none.
fn remove_timings(asset: &Asset, project_root: &Path) -> Result<u64, GcError> {
    let Some(words) = Words::file_of(asset) else {
        return Ok(0);
    };
    let Ok(file) = words.resolve(project_root) else {
        return Ok(0);
    };
    let Ok(metadata) = fs::metadata(&file) else {
        return Ok(0);
    };
    fs::remove_file(&file).map_err(|source| GcError::Undeletable {
        path: file.clone(),
        source,
    })?;
    Ok(metadata.len())
}

/// Whether a collection may delete what was paid for ([`Asset::is_paid`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paid {
    /// Refuse a paid asset ([`GcError::Paid`]). The answer whenever the user
    /// has not asked for paid media to be deleted.
    Keep,
    /// Delete it like any other — only because the user explicitly asked for
    /// paid media to go.
    Delete,
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
    /// Collecting it would leave a clip, or another asset, pointing at
    /// nothing, which is exactly what validation exists to prevent.
    #[error("asset `{id}` is still used by a clip or named by another asset")]
    StillReferenced {
        /// The asset still referred to.
        id: AssetId,
    },
    /// A page's text names its file, so collecting it would break the page
    /// with nothing to say so ([`named_by_pages`]).
    #[error("asset `{id}` is named by page `{page}`")]
    NamedByPage {
        /// The asset the page names.
        id: AssetId,
        /// The page naming it.
        page: AssetId,
    },
    /// It was paid for, and the caller did not pass [`Paid::Delete`].
    #[error("asset `{id}` was paid for; it is deleted only when the user asks for that")]
    Paid {
        /// The paid asset.
        id: AssetId,
    },
    /// The project without these assets would not load — a reference this
    /// collection does not know about. Nothing was deleted.
    #[error("collecting these would leave a project that does not load: {0}")]
    Refused(#[from] ValidationErrors),
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
