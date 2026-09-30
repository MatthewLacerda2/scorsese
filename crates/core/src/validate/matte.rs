//! Matte checks: every `matte` names a clip that can be one.
//!
//! Whether a matte and the clip it masks are ever on screen together is not
//! here. Two clips that never overlap are a coherent document that draws
//! nothing — `scorsese check` warns about that, and validation refuses only
//! what cannot be drawn at all.

use std::collections::HashMap;

use crate::project::Project;
use crate::timeline::{Clip, ClipId, Track, TrackKind};

use super::error::MatteProblem;
use super::group::scopes;

pub(super) fn check(project: &Project) -> Vec<MatteProblem> {
    let anywhere: HashMap<&ClipId, &Clip> = project
        .every_clip()
        .map(|(_, clip)| (&clip.id, clip))
        .collect();
    let mut errors = Vec::new();
    for scope in scopes(project) {
        let here: HashMap<&ClipId, (&Track, &Clip)> = scope
            .iter()
            .flat_map(|track| {
                track
                    .clips
                    .iter()
                    .map(move |clip| (&clip.id, (track, clip)))
            })
            .collect();
        for (track, clip) in here.values() {
            let Some(matte) = &clip.matte else {
                continue;
            };
            if let Some(problem) = problem(clip, track, &matte.clip, &here, &anywhere) {
                errors.push(problem);
            }
        }
    }
    // A map's order is not the document's, and a report that shuffles between
    // two runs of the same file reads as two different answers.
    errors.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    errors
}

/// What is wrong with `clip`, on `track`, being masked by `named` — the first
/// thing, since each rules out asking the next.
fn problem(
    clip: &Clip,
    track: &Track,
    named: &ClipId,
    here: &HashMap<&ClipId, (&Track, &Clip)>,
    anywhere: &HashMap<&ClipId, &Clip>,
) -> Option<MatteProblem> {
    let masked = clip.id.clone();
    if *named == clip.id {
        return Some(MatteProblem::Itself { clip: masked });
    }
    let Some((matte_track, matte)) = here.get(named) else {
        return Some(if anywhere.contains_key(named) {
            MatteProblem::Across {
                clip: masked,
                matte: named.clone(),
            }
        } else {
            MatteProblem::Missing {
                clip: masked,
                matte: named.clone(),
            }
        });
    };
    let on_sound = [(track, clip), (matte_track, matte)]
        .into_iter()
        .find(|(track, _)| track.kind == TrackKind::Audio);
    if let Some((_, on_sound)) = on_sound {
        return Some(MatteProblem::NotPicture {
            clip: masked,
            matte: named.clone(),
            on_sound: on_sound.id.clone(),
        });
    }
    matte.matte.as_ref().map(|next| MatteProblem::Chained {
        clip: masked,
        matte: named.clone(),
        next: next.clip.clone(),
    })
}
