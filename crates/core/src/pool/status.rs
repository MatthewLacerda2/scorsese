//! What state each asset is actually in.

use std::collections::HashMap;
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
    /// How many clips reference this asset, members of a group included. Zero
    /// means `gc` would collect it — unless [`Self::sequence`] is set, because
    /// a still a sequence plays is in use whether or not a clip shows it.
    pub clip_count: usize,
    /// The image sequence this still is listed under, when one plays it.
    ///
    /// A listing shows a sequence's stills under the sequence rather than
    /// beside it (#684): a 400-photo timelapse is one row, not 401. A still
    /// several sequences play belongs to the first of them in table order, so
    /// it is never listed twice; one a clip *also* uses directly stays there
    /// too, and says so through a non-zero [`Self::clip_count`].
    pub sequence: Option<AssetId>,
}

/// Reports every asset in the project, in table order.
pub fn asset_status(project: &Project, project_root: &Path, check: HashCheck) -> Vec<AssetStatus> {
    let owners = owners(project);
    project
        .assets
        .iter()
        .map(|asset| AssetStatus {
            id: asset.id.clone(),
            kind: asset.kind,
            health: health_of(asset, project_root, check),
            clip_count: project
                .every_clip()
                .filter(|(_, clip)| clip.asset == asset.id)
                .count(),
            sequence: owners.get(&asset.id).map(|&owner| owner.clone()),
        })
        .collect()
}

/// Which sequence each still is listed under: the first, in table order, that
/// plays it. Only an `image` is ever owned — validation already refuses a
/// sequence naming anything else, and a listing should not hide a row because
/// a broken document claimed it.
fn owners(project: &Project) -> HashMap<&AssetId, &AssetId> {
    let mut owners = HashMap::new();
    for asset in &project.assets {
        let Some(sequence) = &asset.sequence else {
            continue;
        };
        for still in &sequence.stills {
            if project.asset(still).map(|still| still.kind) == Some(AssetKind::Image) {
                owners.entry(still).or_insert(&asset.id);
            }
        }
    }
    owners
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

    // A page is never probed — there is no stream in it to measure — so the
    // absence of a `media` block is what a healthy one looks like.
    if asset.media.is_none() && asset.kind.is_media() {
        return AssetHealth::Unprobed;
    }
    AssetHealth::Ok
}
