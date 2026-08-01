//! Track and clip checks: identity, references, time, overlap, keyframes.

use std::collections::HashSet;

use crate::asset::Asset;
use crate::project::Project;
use crate::time::{Fps, Frames};
use crate::timeline::{Clip, Track, TrackKind};

use super::error::TimelineProblem;

pub(super) fn check(project: &Project) -> Vec<TimelineProblem> {
    let mut errors = Vec::new();
    let mut track_ids = HashSet::new();
    let mut clip_ids = HashSet::new();
    let (mut tracks_reported, mut clips_reported) = (HashSet::new(), HashSet::new());

    for track in &project.tracks {
        if !track_ids.insert(&track.id) && tracks_reported.insert(&track.id) {
            errors.push(TimelineProblem::DuplicateTrackId {
                id: track.id.clone(),
            });
        }
        for clip in &track.clips {
            if !clip_ids.insert(&clip.id) && clips_reported.insert(&clip.id) {
                errors.push(TimelineProblem::DuplicateClipId {
                    id: clip.id.clone(),
                });
            }
            if let Some(asset) = check_reference(project, track, clip, &mut errors) {
                check_source_range(project.timeline_fps, asset, clip, &mut errors);
            }
            check_duration(clip, &mut errors);
            check_crop(clip, &mut errors);
            check_keyframes(clip, &mut errors);
        }
        check_overlaps(track, &mut errors);
    }
    errors
}

/// A clip names an asset by id; the id has to exist, and the asset it names
/// has to make sense on the kind of track the clip sits on.
///
/// Hands the asset back, because the checks that follow are about the clip
/// *and* what it shows — and a clip whose reference does not resolve has
/// nothing to check them against.
fn check_reference<'a>(
    project: &'a Project,
    track: &Track,
    clip: &Clip,
    errors: &mut Vec<TimelineProblem>,
) -> Option<&'a Asset> {
    let Some(asset) = project.asset(&clip.asset) else {
        errors.push(TimelineProblem::DanglingAssetRef {
            clip: clip.id.clone(),
            asset: clip.asset.clone(),
        });
        return None;
    };
    let fits = match track.kind {
        TrackKind::Video => asset.kind.is_visual(),
        TrackKind::Audio => asset.kind.is_audible(),
    };
    if !fits {
        errors.push(TimelineProblem::TrackKindMismatch {
            track: track.id.clone(),
            track_kind: track.kind,
            clip: clip.id.clone(),
            asset_kind: asset.kind,
        });
    }
    Some(asset)
}

/// Frame times are whole and non-negative by construction, so the only way a
/// clip's timing is wrong is that it covers no frame at all. A negative or
/// fractional time never reaches validation — it fails to parse.
fn check_duration(clip: &Clip, errors: &mut Vec<TimelineProblem>) {
    if clip.duration == Frames::ZERO {
        errors.push(TimelineProblem::ZeroDuration {
            clip: clip.id.clone(),
        });
    }
}

/// A clip cannot show footage nobody shot: where it starts in the source plus
/// what it plays through has to land inside the source's own length.
///
/// The one check here that reads something measured rather than something
/// written, and so the one that can only be made when a measurement exists.
/// [`Asset::length`] answers `None` for everything with no inherent length —
/// a still, a title, a colour, a sketch with no file yet — and for a file
/// nobody has probed, and an absent ceiling bounds nothing. That is the rule
/// being honest about what it knows: the alternative, guessing a length, would
/// refuse honest edits on assets we have simply not looked at.
fn check_source_range(fps: Fps, asset: &Asset, clip: &Clip, errors: &mut Vec<TimelineProblem>) {
    let Some(available) = asset.length(fps) else {
        return;
    };
    let reaches = clip.source_end();
    if reaches > available {
        errors.push(TimelineProblem::ClipOutlastsSource {
            clip: clip.id.clone(),
            asset: asset.id.clone(),
            reaches,
            available,
        });
    }
}

/// A crop names a rectangle of the source in fractions of it, so a rectangle
/// that runs off an edge or encloses nothing is checkable here — from the
/// document alone, without knowing how big the source is. That the source
/// *exists* is the render's to find out, as it is for every path.
fn check_crop(clip: &Clip, errors: &mut Vec<TimelineProblem>) {
    let Some(crop) = clip.crop else {
        return;
    };
    if !crop.is_within_source() {
        errors.push(TimelineProblem::CropOutsideSource {
            clip: clip.id.clone(),
            rectangle: crop
                .edges()
                .map(|(field, value)| format!("{field} {value}"))
                .join(", "),
        });
    }
}

/// Clips on one track may touch but never overlap: at any instant a track
/// shows exactly one clip, which is what lets the compositor walk tracks
/// bottom to top without resolving conflicts.
fn check_overlaps(track: &Track, errors: &mut Vec<TimelineProblem>) {
    let mut ordered: Vec<&Clip> = track.clips.iter().collect();
    ordered.sort_by_key(|clip| clip.start);

    for pair in ordered.windows(2) {
        let (first, second) = (pair[0], pair[1]);
        if first.overlaps(second) {
            errors.push(TimelineProblem::OverlappingClips {
                track: track.id.clone(),
                first: first.id.clone(),
                second: second.id.clone(),
            });
        }
    }
}

/// Keyframe tracks are checked for shape only — that the property path parses
/// and the times ascend. Whether the property *exists* is deliberately not
/// checked here: core defines property types, never property values.
fn check_keyframes(clip: &Clip, errors: &mut Vec<TimelineProblem>) {
    for track in &clip.keyframes {
        let property = || track.property.clone();
        let clip_id = || clip.id.clone();

        if !track.property.is_well_formed() {
            errors.push(TimelineProblem::MalformedPropertyPath {
                clip: clip_id(),
                property: property(),
            });
        }
        // A blank signature is worse than none: it claims a tool wrote this
        // and names no tool, so nothing can ever recognise or replace it.
        // Whether the tool *exists* is not checked, for the same reason a
        // property path is a string — a document written against a newer
        // scorsese still has to load on an older one.
        if track.by.as_deref().is_some_and(|by| by.trim().is_empty()) {
            errors.push(TimelineProblem::BlankKeyframeAuthor {
                clip: clip_id(),
                property: property(),
            });
        }
        if track.keyframes.is_empty() {
            errors.push(TimelineProblem::EmptyKeyframeTrack {
                clip: clip_id(),
                property: property(),
            });
            continue;
        }
        if !track.is_sorted() {
            errors.push(TimelineProblem::UnsortedKeyframes {
                clip: clip_id(),
                property: property(),
            });
        }
        for keyframe in &track.keyframes {
            if !keyframe.value.is_finite() {
                errors.push(TimelineProblem::BadKeyframeValue {
                    clip: clip_id(),
                    property: property(),
                });
            }
        }
    }
}
