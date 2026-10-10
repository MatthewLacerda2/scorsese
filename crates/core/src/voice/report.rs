//! What a cut says about the clips it did not lay out itself.

use std::collections::BTreeSet;

use super::Voicing;
use super::lay::Plan;
use crate::asset::AssetKind;
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{Clip, ClipId};
use crate::{captions, dip};

/// Clips the call did not name whose scenes changed under them: before, they
/// overlapped one set of scenes, and now another. A music bed that ran the
/// whole cut and still does is not listed; one the cut now runs past is.
pub(super) fn crossed(
    before: &Project,
    after: &Project,
    voicing: &Voicing,
    plan: &Plan,
) -> Vec<ClipId> {
    let named = named(voicing);
    let under = |clip: &Clip, spans: &[(Frames, Frames)]| -> Vec<usize> {
        spans
            .iter()
            .enumerate()
            .filter(|(_, (start, end))| clip.start < *end && *start < clip.end())
            .map(|(index, _)| index)
            .collect()
    };
    before
        .clips()
        .map(|(_, clip)| clip)
        .filter(|clip| !named.contains(&clip.id))
        .filter(|clip| !clip.id.as_str().starts_with(captions::PREFIX))
        .filter(|clip| {
            let now = after.clips().find(|(_, other)| other.id == clip.id);
            now.is_some_and(|(_, now)| under(clip, &plan.before) != under(now, &plan.after))
        })
        .map(|clip| clip.id.clone())
        .collect()
}

/// Visuals with a keyframe past their new end.
pub(super) fn keyed_past_end(project: &Project, voicing: &Voicing) -> Vec<ClipId> {
    let visuals: BTreeSet<&ClipId> = voicing
        .scenes
        .iter()
        .flat_map(|scene| &scene.visuals)
        .collect();
    project
        .clips()
        .map(|(_, clip)| clip)
        .filter(|clip| visuals.contains(&clip.id))
        .filter(|clip| {
            clip.keyframes
                .iter()
                .flat_map(|track| &track.keyframes)
                .any(|key| key.t > clip.duration)
        })
        .map(|clip| clip.id.clone())
        .collect()
}

/// What else in the project was worked out from the old positions.
pub(super) fn follow_ups(project: &Project) -> Vec<&'static str> {
    let clips = || project.clips().map(|(_, clip)| clip);
    let mut follow = Vec::new();
    if clips().any(|clip| clip.id.as_str().starts_with(captions::PREFIX)) {
        follow.push("captions were timed from the old positions: run `caption_narration` again");
    }
    if clips().any(|clip| clip.keyframes.iter().any(|k| k.is_generated_by(dip::TOOL))) {
        follow.push("the music was ducked under the old positions: run `duck_music` again");
    }
    let scored = clips().any(|clip| {
        project
            .asset(&clip.asset)
            .is_some_and(|asset| asset.kind == AssetKind::SynthAudio)
    });
    if scored {
        follow
            .push("a synthesized score fitted to its clip re-fits at its next bake: bake it again");
    }
    follow
}

/// Every clip the call names.
fn named(voicing: &Voicing) -> BTreeSet<ClipId> {
    voicing
        .scenes
        .iter()
        .flat_map(|scene| {
            std::iter::once(&scene.line)
                .chain(&scene.visuals)
                .chain(&scene.riders)
        })
        .cloned()
        .collect()
}
