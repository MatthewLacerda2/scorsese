//! Opening a group: which of its members are on screen at an instant, and
//! where they cut the timeline.
//!
//! A group clip is one layer, but the stretch it covers is not one unchanging
//! picture: a member entering or leaving inside the group changes what that
//! layer shows. So a group's member boundaries are cuts on the timeline exactly
//! as a clip's own are — mapped out through the group clip's place and trim,
//! and only where the group clip is actually showing them — and within a
//! stretch the set of members is as fixed as the set of layers.
//!
//! **Group time.** Every member counts its `start` and its keyframes from the
//! group's zero, and a clip of the group is a window onto that time opening at
//! its `source_in`. So a member's track runs `shift` frames behind the
//! timeline, where `shift` is where the group's zero lands: the group clip's
//! `start` less its `source_in`, plus whatever its own track was already
//! behind. A group clip always plays at one group frame per timeline frame —
//! validation refuses a speed on one — which is what lets this be an offset
//! and never a rate.

use std::collections::HashSet;

use scorsese_core::{Asset, AssetId, AssetKind, Clip, Frames, Project, Track};

use super::segments::{carries_sound, renderable_asset, showing, source_in_at};
use super::{PlanError, Shot};
use crate::report::Note;

/// What `track` shows at timeline frame `at`, when the track runs `shift`
/// frames behind the timeline — a group's members opened beneath it, and
/// theirs beneath them.
///
/// `open` is the groups being opened on the way down, which is how a group
/// containing itself stops the render rather than recursing until the stack
/// runs out: validation refuses one, and a document nobody validated is
/// refused here instead.
pub(super) fn shot_at<'a>(
    project: &'a Project,
    track: &'a Track,
    at: Frames,
    shift: i64,
    keep: &impl Fn(&Track, &Clip) -> bool,
    open: &mut Vec<&'a AssetId>,
) -> Result<Option<Shot<'a>>, PlanError> {
    let Some(local) = on_track(at, shift) else {
        return Ok(None);
    };
    let Some(clip) = track
        .clips
        .iter()
        .find(|clip| clip.start <= local && local < clip.end() && keep(track, clip))
    else {
        return Ok(None);
    };
    let asset = renderable_asset(project, clip)?;
    let mut members = Vec::new();
    if let Some(group) = group_of(asset) {
        if open.contains(&&asset.id) {
            return Err(PlanError::GroupContainsItself {
                asset: asset.id.to_string(),
            });
        }
        open.push(&asset.id);
        let inner = zero_of(shift, clip);
        for lane in &group.tracks {
            members.extend(shot_at(project, lane, at, inner, keep, open)?);
        }
        members = super::mattes::attach(&group.tracks, members);
        open.pop();
    }
    let stills = stills_of(project, clip, asset)?;
    Ok(Some(Shot {
        track: &track.id,
        clip,
        asset,
        showing: showing(asset),
        source_in: source_in_at(clip, local),
        members,
        stills,
        shift,
        matte: None,
    }))
}

/// The stills a clip of an image sequence plays, each one an imported picture
/// with a file — refused otherwise, since validation refuses the same, and a
/// sequence with a hole in it has no frame to show there.
fn stills_of<'a>(
    project: &'a Project,
    clip: &Clip,
    asset: &Asset,
) -> Result<Vec<&'a Asset>, PlanError> {
    let Some(sequence) = &asset.sequence else {
        return Ok(Vec::new());
    };
    let refuse = |still: &AssetId| PlanError::UnplayableStill {
        clip: clip.id.to_string(),
        asset: asset.id.to_string(),
        still: still.to_string(),
    };
    let stills = sequence
        .stills
        .iter()
        .map(|id| match project.asset(id) {
            Some(still) if still.kind == AssetKind::Image && still.path.is_some() => Ok(still),
            _ => Err(refuse(id)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if stills.is_empty() {
        return Err(PlanError::NoMedia {
            clip: clip.id.to_string(),
            asset: asset.id.to_string(),
        });
    }
    Ok(stills)
}

/// Every timeline frame at which what `track` shows can change, members of
/// its groups included — each group's only inside the stretch its clip shows.
///
/// Signed, because a member's boundary can fall before the timeline's zero
/// when a group clip opens part way into its group; the caller keeps only the
/// ones inside the render.
pub(super) fn boundaries(
    project: &Project,
    track: &Track,
    shift: i64,
    keep: &impl Fn(&Track, &Clip) -> bool,
    open: &mut Vec<AssetId>,
    out: &mut Vec<i64>,
) {
    for clip in track.clips.iter().filter(|clip| keep(track, clip)) {
        let from = signed(clip.start) + shift;
        let to = signed(clip.end()) + shift;
        out.extend([from, to]);
        let Some(asset) = project.asset(&clip.asset) else {
            continue;
        };
        let Some(group) = group_of(asset) else {
            continue;
        };
        // A group on the way down again is a cycle, which [`shot_at`] refuses
        // with a reason; here it only has to stop.
        if open.contains(&asset.id) {
            continue;
        }
        open.push(asset.id.clone());
        let mut inner = Vec::new();
        for lane in &group.tracks {
            boundaries(project, lane, zero_of(shift, clip), keep, open, &mut inner);
        }
        open.pop();
        out.extend(inner.into_iter().filter(|cut| from < *cut && *cut < to));
    }
}

/// A note for every member of a placed group whose own file has sound on it,
/// which a group does not mix.
pub(super) fn silenced(project: &Project) -> Vec<Note> {
    let placed: HashSet<&AssetId> = project.every_clip().map(|(_, clip)| &clip.asset).collect();
    project
        .assets
        .iter()
        .filter(|asset| placed.contains(&asset.id))
        .filter_map(|asset| group_of(asset).map(|group| (asset, group)))
        .flat_map(|(asset, group)| {
            group
                .clips()
                .filter(|(_, clip)| project.asset(&clip.asset).is_some_and(carries_sound))
                .map(|(_, clip)| Note::GroupedSoundLeftOut {
                    clip: clip.id.to_string(),
                    group: asset.id.to_string(),
                })
        })
        .collect()
}

/// The group an asset is, if it is one.
fn group_of(asset: &scorsese_core::Asset) -> Option<&scorsese_core::Group> {
    asset
        .group
        .as_ref()
        .filter(|_| asset.kind == AssetKind::Group)
}

/// Where a group clip's group has its zero, as a shift behind the timeline:
/// the clip's start on its own track, less how far into the group it opens.
fn zero_of(shift: i64, clip: &Clip) -> i64 {
    shift + signed(clip.start) - signed(clip.source_in)
}

/// Frame `at` of the timeline on a track running `shift` behind it, if that is
/// not before the track's own zero.
fn on_track(at: Frames, shift: i64) -> Option<Frames> {
    u64::try_from(signed(at) - shift).ok().map(Frames)
}

/// A frame count as a signed number. A timeline long enough to overflow this
/// is some three hundred million years of video.
fn signed(frames: Frames) -> i64 {
    i64::try_from(frames.get()).unwrap_or(i64::MAX)
}
