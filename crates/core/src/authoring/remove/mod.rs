//! Taking an asset, or a lane, out of the document — and the clips that go
//! with it, **only when they are named**.
//!
//! An asset is an entity and clips are references to it, so removing one has
//! to say what becomes of the clips showing it. The answer here (#396) is that
//! they go too, but never as a side effect of naming something else: the
//! caller passes the **exact** list of clip ids that will be lost, and a list
//! that is missing one, or names one that is not lost, is refused with the
//! right list in the refusal. Confirming means naming what will be destroyed,
//! which is why this is a list and not a boolean — a flag is how a caller comes
//! to pick the destructive behaviour by not thinking about it.
//!
//! A lane is the same question one level up, and has the same answer.
//!
//! **Both are all-or-nothing** over [`Project::validate`], like every other
//! operation in [`crate::authoring`]: a removal that would leave anything
//! pointing at nothing — an arrow attached to a clip that went, a generated
//! shot whose first frame was the image removed — is refused by validation and
//! leaves the project as it was.
//!
//! **The table entry, not the file.** An imported asset's copy in `assets/`
//! and a generated one's output in `generated/` stay on disk: this crate's
//! authoring never touches a directory, and a generated output kept by the
//! hash of its brief is exactly what lets putting the asset back cost nothing.

use std::collections::BTreeSet;

use super::AuthorError;
use crate::asset::{Asset, AssetId};
use crate::placing::Removed;
use crate::project::Project;
use crate::timeline::{ClipId, Track, TrackId};

/// What [`remove_asset`] took out of the document.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetRemoval {
    /// The asset as it was, so a reply can say what went.
    pub asset: Asset,
    /// Every clip that showed it, with the track each was on — the timeline's
    /// and every group's lanes alike.
    pub clips: Vec<Removed>,
}

/// Removes an asset and every clip that shows it — which have to be exactly
/// `clips`.
///
/// An asset no clip shows goes with an empty list. One that is shown goes only
/// when `clips` is that set of ids, no more and no fewer; otherwise
/// [`AuthorError::AssetInUse`] says which clips those are, so the next call can
/// name them once somebody has agreed to lose them. A still an image sequence
/// plays is refused whatever is named ([`AuthorError::PlayedBySequence`]): the
/// sequence is not a clip, and has to let go of it first.
pub fn remove_asset(
    project: &mut Project,
    asset: &AssetId,
    clips: &BTreeSet<ClipId>,
) -> Result<AssetRemoval, AuthorError> {
    if !project.assets.iter().any(|entry| &entry.id == asset) {
        return Err(AuthorError::NoSuchAsset {
            asset: asset.clone(),
        });
    }
    let sequences: Vec<AssetId> = project
        .assets
        .iter()
        .filter(|entry| {
            entry
                .sequence
                .as_ref()
                .is_some_and(|sequence| sequence.stills.contains(asset))
        })
        .map(|entry| entry.id.clone())
        .collect();
    if !sequences.is_empty() {
        return Err(AuthorError::PlayedBySequence {
            asset: asset.clone(),
            sequences,
        });
    }
    let using: BTreeSet<ClipId> = project
        .every_clip()
        .filter(|(_, clip)| &clip.asset == asset)
        .map(|(_, clip)| clip.id.clone())
        .collect();
    if &using != clips {
        return Err(AuthorError::AssetInUse {
            asset: asset.clone(),
            using: using.into_iter().collect(),
            named: clips.iter().cloned().collect(),
        });
    }

    let mut proposed = project.clone();
    let mut removed = Vec::new();
    for track in every_track_mut(&mut proposed) {
        let (gone, kept) = std::mem::take(&mut track.clips)
            .into_iter()
            .partition(|clip| &clip.asset == asset);
        track.clips = kept;
        removed.extend(gone.into_iter().map(|clip| Removed {
            track: track.id.clone(),
            clip,
        }));
    }
    let index = proposed
        .assets
        .iter()
        .position(|entry| &entry.id == asset)
        .expect("the asset was found in the document this is a copy of");
    let entry = proposed.assets.remove(index);

    proposed.validate()?;
    *project = proposed;
    Ok(AssetRemoval {
        asset: entry,
        clips: removed,
    })
}

/// Removes a track — the timeline's or a group's lane — and the clips on it,
/// which have to be exactly `clips`. Hands back the track as it was, clips and
/// all.
///
/// An empty lane goes with an empty list; one holding clips goes only when
/// `clips` names every one of them and nothing else, for
/// [`remove_asset`]'s reason.
pub fn remove_track(
    project: &mut Project,
    track: &TrackId,
    clips: &BTreeSet<ClipId>,
) -> Result<Track, AuthorError> {
    let Some(lane) = project.every_track().find(|lane| &lane.id == track) else {
        return Err(AuthorError::NoSuchTrack {
            track: track.clone(),
        });
    };
    let holding: BTreeSet<ClipId> = lane.clips.iter().map(|clip| clip.id.clone()).collect();
    if &holding != clips {
        return Err(AuthorError::TrackNotEmpty {
            track: track.clone(),
            holding: holding.into_iter().collect(),
            named: clips.iter().cloned().collect(),
        });
    }

    let mut proposed = project.clone();
    let mut taken = None;
    proposed.tracks.retain(|lane| keep(lane, track, &mut taken));
    for group in proposed
        .assets
        .iter_mut()
        .filter_map(|asset| asset.group.as_mut())
    {
        group.tracks.retain(|lane| keep(lane, track, &mut taken));
    }
    let taken = taken.expect("the track was found in the document this is a copy of");

    proposed.validate()?;
    *project = proposed;
    Ok(taken)
}

/// `retain`'s predicate: keeps every lane but `track`, which it hands to
/// `taken` instead.
fn keep(lane: &Track, track: &TrackId, taken: &mut Option<Track>) -> bool {
    if &lane.id == track {
        *taken = Some(lane.clone());
        return false;
    }
    true
}

/// Every track in the document, mutably: the timeline's, then each group's —
/// [`Project::every_track`]'s order.
fn every_track_mut(project: &mut Project) -> impl Iterator<Item = &mut Track> {
    let lanes = project
        .assets
        .iter_mut()
        .filter_map(|asset| asset.group.as_mut())
        .flat_map(|group| group.tracks.iter_mut());
    project.tracks.iter_mut().chain(lanes)
}

/// Clip ids as a refusal reads them: `` `a`, `b` ``, or `none` for an empty
/// list, so "you named none" is a sentence rather than a blank.
pub(super) fn listed(ids: &[ClipId]) -> String {
    if ids.is_empty() {
        return "none".to_owned();
    }
    ids.iter()
        .map(|id| format!("`{id}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
