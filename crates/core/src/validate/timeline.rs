//! Track and clip checks: identity, references, time, overlap, keyframes.

use std::collections::HashSet;

use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{Clip, Track, TrackKind};

use super::error::ValidationError;

pub(super) fn check(project: &Project, errors: &mut Vec<ValidationError>) {
    let mut track_ids = HashSet::new();
    let mut clip_ids = HashSet::new();
    let (mut tracks_reported, mut clips_reported) = (HashSet::new(), HashSet::new());

    for track in &project.tracks {
        if !track_ids.insert(&track.id) && tracks_reported.insert(&track.id) {
            errors.push(ValidationError::DuplicateTrackId {
                id: track.id.clone(),
            });
        }
        for clip in &track.clips {
            if !clip_ids.insert(&clip.id) && clips_reported.insert(&clip.id) {
                errors.push(ValidationError::DuplicateClipId {
                    id: clip.id.clone(),
                });
            }
            check_reference(project, track, clip, errors);
            check_duration(clip, errors);
            check_keyframes(clip, errors);
        }
        check_overlaps(track, errors);
    }
}

/// A clip names an asset by id; the id has to exist, and the asset it names
/// has to make sense on the kind of track the clip sits on.
fn check_reference(
    project: &Project,
    track: &Track,
    clip: &Clip,
    errors: &mut Vec<ValidationError>,
) {
    let Some(asset) = project.asset(&clip.asset) else {
        errors.push(ValidationError::DanglingAssetRef {
            clip: clip.id.clone(),
            asset: clip.asset.clone(),
        });
        return;
    };
    let fits = match track.kind {
        TrackKind::Video => asset.kind.is_visual(),
        TrackKind::Audio => asset.kind.is_audible(),
    };
    if !fits {
        errors.push(ValidationError::TrackKindMismatch {
            track: track.id.clone(),
            track_kind: track.kind,
            clip: clip.id.clone(),
            asset_kind: asset.kind,
        });
    }
}

/// Frame times are whole and non-negative by construction, so the only way a
/// clip's timing is wrong is that it covers no frame at all. A negative or
/// fractional time never reaches validation — it fails to parse.
fn check_duration(clip: &Clip, errors: &mut Vec<ValidationError>) {
    if clip.duration == Frames::ZERO {
        errors.push(ValidationError::ZeroDuration {
            clip: clip.id.clone(),
        });
    }
}

/// Clips on one track may touch but never overlap: at any instant a track
/// shows exactly one clip, which is what lets the compositor walk tracks
/// bottom to top without resolving conflicts.
fn check_overlaps(track: &Track, errors: &mut Vec<ValidationError>) {
    let mut ordered: Vec<&Clip> = track.clips.iter().collect();
    ordered.sort_by_key(|clip| clip.start);

    for pair in ordered.windows(2) {
        let (first, second) = (pair[0], pair[1]);
        if first.overlaps(second) {
            errors.push(ValidationError::OverlappingClips {
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
fn check_keyframes(clip: &Clip, errors: &mut Vec<ValidationError>) {
    for track in &clip.keyframes {
        let property = || track.property.clone();
        let clip_id = || clip.id.clone();

        if !track.property.is_well_formed() {
            errors.push(ValidationError::MalformedPropertyPath {
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
            errors.push(ValidationError::BlankKeyframeAuthor {
                clip: clip_id(),
                property: property(),
            });
        }
        if track.keyframes.is_empty() {
            errors.push(ValidationError::EmptyKeyframeTrack {
                clip: clip_id(),
                property: property(),
            });
            continue;
        }
        if !track.is_sorted() {
            errors.push(ValidationError::UnsortedKeyframes {
                clip: clip_id(),
                property: property(),
            });
        }
        for keyframe in &track.keyframes {
            if !keyframe.value.is_finite() {
                errors.push(ValidationError::BadKeyframeValue {
                    clip: clip_id(),
                    property: property(),
                });
            }
        }
    }
}
