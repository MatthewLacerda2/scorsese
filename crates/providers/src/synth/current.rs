//! Which baked songs and effects no longer match what they were made from.
//!
//! A render plays the file an asset's `path` names and never bakes: baking is
//! its own step, deliberately, so a render's cost is the render's. That leaves
//! one way for a cut to sound wrong with nothing to say so — an asset baked
//! before its recipe was edited, before a patch file it names changed, before
//! the synthesiser moved, or, since #1000, before the clip it is fitted to
//! changed length. The music then ends early or is cut off.
//!
//! The answer is cheap, because the bake's address already *is* the record of
//! what made it: read the recipe the way a bake would, work out the address,
//! and compare it with the path the asset holds. Nothing is rendered.
//!
//! It is also where a clip **longer than its bake** is said. Validation leaves
//! a synth asset's clips unbounded, because the document alone cannot tell a
//! stale bake's length from a current one's (#1062); here it can. A stale bake
//! is reported as stale — baking again is the fix, and may well make it long
//! enough — and only a current one is held to its length, since then nothing
//! but a longer recipe will fill the clip.

use std::path::Path;

use scorsese_core::{Asset, GenerationState, Project};

use super::{Read, address, named_patches, read_recipe};

/// One line per synth asset a render would play out of date, or would run out
/// of under a clip, worded for the person or agent about to hear it, and
/// naming the step that fixes it.
///
/// Only assets a clip actually plays, and only ones that claim a bake: a
/// sketch already renders as a slug card that says so, and an asset nothing
/// plays cannot be heard wrong. A recipe that can no longer be read the way a
/// bake reads it is reported with the reason the bake would give.
pub fn out_of_date(project: &Project, project_root: &Path) -> Vec<String> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind.is_synthesized())
        .filter(|asset| asset.state == Some(GenerationState::Generated))
        .filter(|asset| project.clips_of(&asset.id).next().is_some())
        .filter_map(|asset| {
            let why = match read_recipe(project, asset, project_root) {
                Ok(Read {
                    recipe,
                    digest,
                    placed,
                    ..
                }) => {
                    let wanted =
                        address::output(&digest, &named_patches(&recipe, project_root), &placed);
                    if asset.path.as_ref() == Some(&wanted) {
                        return outlasted(project, asset);
                    }
                    "its bake is older than its recipe, a patch it names, the synthesiser or \
                     the length of the clip it is fitted to"
                        .to_owned()
                }
                Err(problem) => format!("it would not bake as it stands: {problem}"),
            };
            Some(format!(
                "synth asset `{}` plays its last bake, and {why} — run `scorsese synth bake` \
                 (`synth_bake`) and render again",
                asset.id
            ))
        })
        .collect()
}

/// The line for a current bake some clip plays past the end of, naming the
/// clip that reaches furthest — the rest of it is silence, so the fix is a
/// longer recipe, not another bake of this one.
fn outlasted(project: &Project, asset: &Asset) -> Option<String> {
    let fps = project.timeline_fps;
    let available = asset.length(fps)?;
    let clip = project
        .clips_of(&asset.id)
        .max_by_key(|clip| clip.source_end())?;
    let reaches = clip.source_end();
    (reaches > available).then(|| {
        format!(
            "synth asset `{}` is baked {:.2} s long and clip `{}` plays it to {:.2} s, so the \
             rest is silence — lengthen the recipe (or fit it to the clip, `\"fit\": \
             {{\"to\": \"clip\"}}`) and run `scorsese synth bake` (`synth_bake`)",
            asset.id,
            fps.seconds(available),
            clip.id,
            fps.seconds(reaches),
        )
    })
}
