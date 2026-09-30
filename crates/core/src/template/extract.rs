//! Lifting chosen clips out of a project, with everything they need.

use std::collections::BTreeSet;

use crate::asset::{AssetId, GenerationState};
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{ClipId, Track};
use crate::validate::ValidationErrors;

/// Why no template was made. Nothing is ever half made.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// No clips were named.
    #[error("a template needs at least one clip")]
    Nothing,
    /// Clips the project does not have.
    #[error("this project has no clip called {}", quoted(.0))]
    NoSuchClips(Vec<ClipId>),
    /// An arrow among the chosen clips follows one that was not chosen, so the
    /// template would hold an arrow pointing at nothing.
    #[error(
        "the arrow `{arrow}` follows clip `{follows}`, which is not among the clips chosen — \
         choose it too, or leave the arrow out"
    )]
    FollowsUnchosen {
        /// The arrow's asset.
        arrow: AssetId,
        /// The clip it follows.
        follows: ClipId,
    },
    /// A chosen clip shows a group. A template does not carry one yet (#598): the
    /// group's members would need their assets, ids and frame rate carried
    /// through [`crate::template::insert`] too, and a half-carried group is a
    /// template that refuses to insert.
    #[error(
        "`{asset}` is a group, and a template cannot carry a group yet — ungroup it first \
         (clip_ungroup), or leave its clip out"
    )]
    CarriesGroup {
        /// The group asset.
        asset: AssetId,
    },
    /// What was lifted out is not a document that loads — which a valid
    /// project cannot produce, and an invalid one can.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

fn quoted(clips: &[ClipId]) -> String {
    let names: Vec<String> = clips.iter().map(|clip| format!("`{clip}`")).collect();
    names.join(", ")
}

/// A template called `name`, made of `clips` and everything they reference.
///
/// Each track holding a chosen clip comes along — its id, kind, name and note —
/// carrying only the chosen clips, moved so the earliest starts at frame zero.
/// Each asset a chosen clip shows comes along, and so does each still a
/// generated video's brief names, so the template stands alone. A brief that
/// was queued is kept as a sketch: a template records what to generate, not
/// work somebody else's project has in flight.
pub fn extract(
    project: &Project,
    clips: &BTreeSet<ClipId>,
    name: &str,
) -> Result<Project, ExtractError> {
    if clips.is_empty() {
        return Err(ExtractError::Nothing);
    }
    let missing: Vec<ClipId> = clips
        .iter()
        .filter(|id| !project.clips().any(|(_, clip)| &clip.id == *id))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(ExtractError::NoSuchClips(missing));
    }
    let chosen = || project.clips().filter(|(_, clip)| clips.contains(&clip.id));
    let opens = chosen()
        .map(|(_, clip)| clip.start)
        .min()
        .unwrap_or(Frames::ZERO);

    let mut template = Project::new(name, project.timeline_fps);
    for track in &project.tracks {
        let kept: Vec<_> = track
            .clips
            .iter()
            .filter(|clip| clips.contains(&clip.id))
            .map(|clip| {
                let mut clip = clip.clone();
                clip.start = Frames(clip.start.get() - opens.get());
                clip
            })
            .collect();
        if kept.is_empty() {
            continue;
        }
        template.tracks.push(Track {
            clips: kept,
            ..Track::new(track.id.clone(), track.kind)
        });
        let lane = template.tracks.last_mut().expect("just pushed");
        lane.name.clone_from(&track.name);
        lane.note.clone_from(&track.note);
    }

    let needed = needed(project, chosen().map(|(_, clip)| clip.asset.clone()));
    for asset in project.assets.iter().filter(|a| needed.contains(&a.id)) {
        if asset.group.is_some() {
            return Err(ExtractError::CarriesGroup {
                asset: asset.id.clone(),
            });
        }
        for attach in super::follows(asset) {
            if !clips.contains(&attach.clip) {
                return Err(ExtractError::FollowsUnchosen {
                    arrow: asset.id.clone(),
                    follows: attach.clip.clone(),
                });
            }
        }
        let mut asset = asset.clone();
        if asset.state == Some(GenerationState::Queued) {
            asset.state = Some(GenerationState::Sketch);
            asset.operation = None;
            asset.queued_at = None;
        }
        template.assets.push(asset);
    }
    template.validate()?;
    Ok(template)
}

/// The assets `shown` names, and every still their briefs name in turn.
fn needed(project: &Project, shown: impl Iterator<Item = AssetId>) -> BTreeSet<AssetId> {
    let mut needed: BTreeSet<AssetId> = shown.collect();
    let stills: Vec<AssetId> = needed
        .iter()
        .filter_map(|id| project.asset(id)?.video.as_ref())
        .flat_map(|brief| brief.images().cloned())
        .collect();
    needed.extend(stills);
    needed
}
