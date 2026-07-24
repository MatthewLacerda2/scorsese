//! What state each asset is actually in.

use std::path::Path;

use crate::asset::{Asset, AssetId, AssetKind, GenerationState};
use crate::project::Project;

use super::hash::hash_file;

/// Whether hashes are re-computed. Verifying reads every file in the pool, so
/// it is asked for rather than assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashCheck {
    Skip,
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
    HashMismatch { recorded: String, found: String },
    /// Present, but nothing has probed it.
    Unprobed,
    /// The file could not be read to verify it.
    Unreadable(String),
}

impl AssetHealth {
    /// True when this asset needs a human or an agent to do something.
    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            Self::Missing | Self::HashMismatch { .. } | Self::Unreadable(_)
        )
    }
}

/// One row of `scorsese assets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetStatus {
    pub id: AssetId,
    pub kind: AssetKind,
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
    if asset.kind == AssetKind::Text {
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
