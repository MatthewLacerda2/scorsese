//! Track and clip checks: identity, references, time, overlap, keyframes.

use std::collections::HashSet;

use crate::project::Project;
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
            check_times(clip, errors);
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

fn check_times(clip: &Clip, errors: &mut Vec<ValidationError>) {
    for (field, value) in [
        ("start", clip.start),
        ("duration", clip.duration),
        ("source_in", clip.source_in),
    ] {
        if !value.is_valid() {
            errors.push(ValidationError::BadTime {
                clip: clip.id.clone(),
                field,
                value: value.get(),
            });
        }
    }
    if clip.duration.is_valid() && clip.duration.get() == 0.0 {
        errors.push(ValidationError::ZeroDuration {
            clip: clip.id.clone(),
        });
    }
}

/// Clips on one track may touch but never overlap: at any instant a track
/// shows exactly one clip, which is what lets the compositor walk tracks
/// bottom to top without resolving conflicts.
fn check_overlaps(track: &Track, errors: &mut Vec<ValidationError>) {
    let mut ordered: Vec<&Clip> = track.clips.iter().filter(|c| c.start.is_valid()).collect();
    ordered.sort_by(|a, b| a.start.cmp_total(b.start));

    for pair in ordered.windows(2) {
        let (first, second) = (pair[0], pair[1]);
        if first.duration.is_valid() && first.overlaps(second) {
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
            if !keyframe.t.is_valid() {
                errors.push(ValidationError::BadKeyframeTime {
                    clip: clip_id(),
                    property: property(),
                    value: keyframe.t.get(),
                });
            }
            if !keyframe.value.is_finite() {
                errors.push(ValidationError::BadKeyframeValue {
                    clip: clip_id(),
                    property: property(),
                });
            }
        }
    }
}
