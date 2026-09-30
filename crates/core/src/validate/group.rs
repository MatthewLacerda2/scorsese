//! Group checks: a group that could never be drawn, and a reference that
//! crosses a group's edge.
//!
//! What a group's *members* must satisfy is not here. They are clips on tracks
//! like any other, so the timeline pass checks them exactly as it checks the
//! project's own — their assets resolve, they do not overlap, their ids are
//! unique across the whole document. What is here is what only a group raises.

use std::collections::HashSet;

use crate::asset::{AssetId, AssetKind};
use crate::project::Project;
use crate::shape::Geometry;
use crate::timeline::{ClipId, Track, TrackKind};

use super::error::GroupProblem;

pub(super) fn check(project: &Project) -> Vec<GroupProblem> {
    let mut errors = Vec::new();
    for asset in project.assets.iter().filter(|a| a.kind == AssetKind::Group) {
        // A missing block is the asset pass's to report, as any missing field is.
        let Some(group) = &asset.group else {
            continue;
        };
        if group.length().get() == 0 {
            errors.push(GroupProblem::Empty {
                asset: asset.id.clone(),
            });
        }
        for track in group.tracks.iter().filter(|t| t.kind == TrackKind::Audio) {
            errors.push(GroupProblem::SoundInGroup {
                asset: asset.id.clone(),
                track: track.id.clone(),
            });
        }
        if contains_itself(project, &asset.id) {
            errors.push(GroupProblem::ContainsItself {
                asset: asset.id.clone(),
            });
        }
    }
    check_speeds(project, &mut errors);
    for scope in scopes(project) {
        check_attachments(project, scope, &mut errors);
    }
    errors
}

/// Every timeline in the document: the project's own, then each group's.
///
/// A **scope** is what an arrow may reach: the clips on the same tracks as
/// itself. Everything else is across an edge.
pub(super) fn scopes(project: &Project) -> impl Iterator<Item = &[Track]> {
    let groups = project
        .assets
        .iter()
        .filter_map(|asset| asset.group.as_ref())
        .map(|group| group.tracks.as_slice());
    std::iter::once(project.tracks.as_slice()).chain(groups)
}

/// Whether a clip of `start` sits anywhere beneath it.
///
/// A walk over group-to-group references, remembering what it has seen so
/// that a cycle *not* through `start` — two other groups containing each
/// other — ends the walk rather than looping; that cycle is reported from the
/// groups on it.
fn contains_itself(project: &Project, start: &AssetId) -> bool {
    let mut stack = vec![start];
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        let Some(group) = project.asset(id).and_then(|asset| asset.group.as_ref()) else {
            continue;
        };
        for (_, clip) in group.clips() {
            if &clip.asset == start {
                return true;
            }
            if seen.insert(&clip.asset) {
                stack.push(&clip.asset);
            }
        }
    }
    false
}

/// A group clip plays at one group frame per timeline frame.
fn check_speeds(project: &Project, errors: &mut Vec<GroupProblem>) {
    for (_, clip) in project.every_clip() {
        let is_group = project
            .asset(&clip.asset)
            .is_some_and(|asset| asset.kind == AssetKind::Group);
        if is_group && !clip.speed.is_normal() {
            errors.push(GroupProblem::AtSpeed {
                clip: clip.id.clone(),
                asset: clip.asset.clone(),
                speed: clip.speed.get(),
            });
        }
    }
}

/// Every arrow clip in `scope` follows only clips in `scope`.
///
/// A target nowhere in the document is not reported here: that is
/// [`super::error::ShapeProblem::AttachedToNothing`], found once per arrow
/// asset rather than once per clip of it.
fn check_attachments(project: &Project, scope: &[Track], errors: &mut Vec<GroupProblem>) {
    let here: HashSet<&ClipId> = scope
        .iter()
        .flat_map(|track| track.clips.iter().map(|clip| &clip.id))
        .collect();
    let anywhere: HashSet<&ClipId> = project.every_clip().map(|(_, clip)| &clip.id).collect();
    for clip in scope.iter().flat_map(|track| &track.clips) {
        let Some(arrow) = project.asset(&clip.asset) else {
            continue;
        };
        let Some(Geometry::Arrow { from, to, .. }) = arrow.shape.as_ref().map(|s| &s.geometry)
        else {
            continue;
        };
        for attach in [from, to].into_iter().filter_map(|end| end.attach()) {
            if !here.contains(&attach.clip) && anywhere.contains(&attach.clip) {
                errors.push(GroupProblem::AttachedAcross {
                    clip: clip.id.clone(),
                    arrow: arrow.id.clone(),
                    target: attach.clip.clone(),
                });
            }
        }
    }
}
