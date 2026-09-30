//! Putting a group's members back onto the timeline.

use crate::asset::{AssetId, AssetKind};
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{Clip, ClipId, TrackId};
use crate::validate::ValidationErrors;

/// What an ungrouping wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Ungrouped {
    /// The group asset, now gone from the table.
    pub asset: AssetId,
    /// Its tracks, now on the timeline directly above where the group clip
    /// was, bottom first.
    pub tracks: Vec<TrackId>,
    /// How many clips came out.
    pub clips: usize,
    /// True when the group clip carried keyframes or effects of its own —
    /// a move, a fade, a blur of the whole group — which applied to the group
    /// as one layer and are **not** carried onto the members.
    pub dropped_its_own_look: bool,
}

/// Why nothing was ungrouped.
#[derive(Debug, thiserror::Error)]
pub enum UngroupError {
    /// The id names no clip on the timeline.
    #[error("no clip on the timeline is called `{clip}`")]
    NoSuchClip {
        /// The id asked for.
        clip: ClipId,
    },
    /// The clip does not show a group.
    #[error("clip `{clip}` shows `{asset}`, which is not a group")]
    NotAGroup {
        /// The clip.
        clip: ClipId,
        /// What it shows.
        asset: AssetId,
    },
    /// The group is placed more than once. Ungrouping one placement would need
    /// a copy of every member — new ids, and new arrows to follow them — and a
    /// copy made without being asked for is a guess.
    #[error(
        "group `{asset}` is also shown by {} — ungrouping one of its clips would have to copy \
         it; remove the others first, or leave it grouped",
        others.iter().map(|c| format!("`{c}`")).collect::<Vec<_>>().join(", ")
    )]
    PlacedElsewhere {
        /// The group.
        asset: AssetId,
        /// Its other clips.
        others: Vec<ClipId>,
    },
    /// The clip shows only part of its group, so some members would come back
    /// partly or wholly outside it.
    #[error(
        "clip `{clip}` shows only frames {from}–{to} of its group, which is {length} long — \
         trim it back to the whole group first, so no member comes out cut"
    )]
    Trimmed {
        /// The clip.
        clip: ClipId,
        /// The first group frame it shows.
        from: Frames,
        /// The frame just past the last one it shows.
        to: Frames,
        /// How long the group is.
        length: Frames,
    },
    /// The result is not a document that loads.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Replaces a group clip with the group's own clips, where the group clip was
/// showing them, and removes the group from the assets table.
///
/// **The group's tracks come back as tracks**, directly above the one the
/// group clip was on and in the same order, so every member is drawn over
/// what it was drawn over inside the group. Their ids come too, and so do the
/// members': nothing that named them has to change.
///
/// **What the group clip did to the group as a whole does not come back.** A
/// scale or a fade on the group clip applied to one layer; applied to each
/// member instead it would scale each about its own centre and fade each
/// through the others, which is a different picture. [`Ungrouped`] says when
/// there was something to lose.
pub fn ungroup(project: &mut Project, clip: &ClipId) -> Result<Ungrouped, UngroupError> {
    let (lane, placed) = project
        .tracks
        .iter()
        .enumerate()
        .find_map(|(at, t)| t.clips.iter().find(|c| &c.id == clip).map(|c| (at, c)))
        .ok_or_else(|| UngroupError::NoSuchClip { clip: clip.clone() })?;
    let placed = placed.clone();
    let asset = project
        .asset(&placed.asset)
        .filter(|asset| asset.kind == AssetKind::Group)
        .ok_or_else(|| UngroupError::NotAGroup {
            clip: clip.clone(),
            asset: placed.asset.clone(),
        })?;
    let group = asset.group.clone().unwrap_or_default();
    let others: Vec<ClipId> = project
        .every_clip()
        .filter(|(_, c)| c.asset == asset.id && &c.id != clip)
        .map(|(_, c)| c.id.clone())
        .collect();
    if !others.is_empty() {
        return Err(UngroupError::PlacedElsewhere {
            asset: asset.id.clone(),
            others,
        });
    }
    let (from, to) = (placed.source_in, placed.source_end());
    if group.clips().any(|(_, c)| c.start < from || c.end() > to) {
        return Err(UngroupError::Trimmed {
            clip: clip.clone(),
            from,
            to,
            length: group.length(),
        });
    }

    let mut proposed = project.clone();
    proposed.tracks[lane].clips.retain(|c| &c.id != clip);
    proposed.assets.retain(|a| a.id != placed.asset);
    let tracks: Vec<TrackId> = group.tracks.iter().map(|t| t.id.clone()).collect();
    let clips = group.clips().count();
    let offset = placed.start.get();
    for (nth, mut track) in group.tracks.into_iter().enumerate() {
        for member in &mut track.clips {
            member.start = Frames(offset + member.start.get() - from.get());
        }
        proposed.tracks.insert(lane + 1 + nth, track);
    }
    proposed.validate()?;
    *project = proposed;
    Ok(Ungrouped {
        asset: placed.asset.clone(),
        tracks,
        clips,
        dropped_its_own_look: has_its_own_look(&placed),
    })
}

/// Whether a clip does anything to its picture beyond placing it in time.
///
/// Answered by comparison with the same placement at every default, so a field
/// added to [`Clip`] later is counted without anyone remembering to list it.
fn has_its_own_look(clip: &Clip) -> bool {
    let plain = Clip {
        note: clip.note.clone(),
        ..Clip::new(
            clip.id.clone(),
            clip.asset.clone(),
            clip.start,
            clip.duration,
        )
    };
    let placed = Clip {
        source_in: Frames::ZERO,
        ..clip.clone()
    };
    placed != plain
}
