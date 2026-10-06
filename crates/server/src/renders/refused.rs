//! What a render job refuses before drawing anything: every asset it cannot
//! supply, by name ([`super::job`] has why).

use std::collections::HashSet;

use scorsese_core::{GenerationState, Project};

use crate::projects::media::Materialised;

/// Refuse, by name, every clip's asset the server cannot supply: a file the
/// library does not hold, or a synthesised sound whose bake it does not. A
/// group's members included — they are drawn, so their files are needed too.
pub(super) fn unrenderable(project: &Project, laid: &Materialised) -> Result<(), String> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    for (_, clip) in project.every_clip() {
        let Some(asset) = project
            .asset(&clip.asset)
            .filter(|_| seen.insert(&clip.asset))
        else {
            continue; // Unknown ids are the renderer's own check to report.
        };
        let absent = laid.missing().contains(&asset.id);
        if asset.kind.is_synthesized() {
            let baked = asset.state == Some(GenerationState::Generated)
                && asset.path.is_some()
                && asset.sha256.is_some();
            if absent || !baked {
                let recipe = asset
                    .recipe
                    .as_ref()
                    .map_or("(none)".into(), ToString::to_string);
                problems.push(format!(
                    "`{}` is synthesised from the recipe {recipe}, and its bake is not in \
                     your library — bake it first (synth_bake)",
                    asset.id
                ));
            }
        } else if absent {
            problems.push(format!(
                "`{}` names a file that is not in your library",
                asset.id
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "cannot render this project: {}",
            problems.join("; ")
        ))
    }
}
