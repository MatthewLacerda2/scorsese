//! What state each asset is actually in.

use std::path::Path;

use crate::asset::{Asset, AssetId, AssetKind, GenerationState};
use crate::project::Project;

use super::hash::hash_file;

/// Whether hashes are re-computed. Verifying reads every file in the pool, so
/// it is asked for rather than assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashCheck {
    /// Trust the recorded hash. A file being *present* is still checked; only
    /// its contents go unread.
    Skip,
    /// Re-hash every file and compare. The slow answer, and the only one that
    /// catches a file edited behind the project's back.
    Verify,
}

/// What is true of one asset right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetHealth {
    /// File present, hash matches (if checked), metadata present.
    Ok,
    /// Content lives in `project.json` itself; there is no file to check.
    Inline,
    /// A prompt that has not become media yet — not a fault.
    Awaiting(GenerationState),
    /// The assets table points at a file that is not there.
    Missing,
    /// The file changed since it was imported.
    HashMismatch {
        /// The hash the assets table remembers from import.
        recorded: String,
        /// The hash the file has now.
        found: String,
    },
    /// Present, but nothing has probed it — so how long it is, how big it is
    /// and whether it carries sound are all unknown.
    Unprobed,
    /// The file could not be read to verify it.
    Unreadable(String),
}

impl AssetHealth {
    /// True when this asset needs a human or an agent to do something.
    ///
    /// [`Self::Unprobed`] is one of them, which it did not used to be. It was
    /// reported as a fact for as long as nothing read the metadata; now that
    /// features do — a source's own length is what bounds a trim — an asset
    /// nobody has looked at is an asset those features silently skip. The
    /// thing to do about it is `scorsese probe`, which is why this counts as
    /// something to fix rather than something to know.
    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            Self::Missing | Self::HashMismatch { .. } | Self::Unreadable(_) | Self::Unprobed
        )
    }
}

/// One row of `scorsese assets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetStatus {
    /// The asset this row is about.
    pub id: AssetId,
    /// Carried along so a report can be read without the project beside it.
    pub kind: AssetKind,
    /// What was found on disk — see [`AssetHealth::needs_attention`] for
    /// which of these are faults rather than facts.
    pub health: AssetHealth,
    /// How many clips reference this asset. Zero means `gc` would collect it.
    pub clip_count: usize,
}

/// Reports every asset in the project, in table order.
pub fn asset_status(project: &Project, project_root: &Path, check: HashCheck) -> Vec<AssetStatus> {
    project
        .assets
        .iter()
        .map(|asset| AssetStatus {
            id: asset.id.clone(),
            kind: asset.kind,
            health: health_of(asset, project_root, check),
            clip_count: project
                .clips()
                .filter(|(_, clip)| clip.asset == asset.id)
                .count(),
        })
        .collect()
}

fn health_of(asset: &Asset, project_root: &Path, check: HashCheck) -> AssetHealth {
    // An inline kind — a title, a colour — is entirely in the document, so
    // there is no file to find and nothing about it can be missing.
    if !asset.kind.is_file_backed() {
        return AssetHealth::Inline;
    }
    match asset.state {
        Some(state) if !state.has_media() => return AssetHealth::Awaiting(state),
        _ => {}
    }

    let Some(path) = &asset.path else {
        return AssetHealth::Missing;
    };
    let file = path.resolve(project_root);
    if !file.is_file() {
        return AssetHealth::Missing;
    }

    if check == HashCheck::Verify
        && let Some(recorded) = &asset.sha256
    {
        match hash_file(&file) {
            Ok(found) if &found != recorded => {
                return AssetHealth::HashMismatch {
                    recorded: recorded.clone(),
                    found,
                };
            }
            Err(error) => return AssetHealth::Unreadable(error.to_string()),
            Ok(_) => {}
        }
    }

    if asset.media.is_none() {
        return AssetHealth::Unprobed;
    }
    AssetHealth::Ok
}
