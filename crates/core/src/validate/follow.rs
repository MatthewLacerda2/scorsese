//! Follow checks: a clip travelling along something that is not a line it can
//! reach.
//!
//! Scoped the way attached arrows are ([`super::group`]): a follower and its
//! arrow sit on the same timeline, the project's own or one group's, because
//! that is the space both are drawn in.

use std::collections::{HashMap, HashSet};

use crate::project::Project;
use crate::shape::Geometry;
use crate::timeline::{Clip, ClipId, Track};

use super::error::FollowProblem;

pub(super) fn check(project: &Project) -> Vec<FollowProblem> {
    let everywhere: HashMap<&ClipId, &Clip> = project
        .every_clip()
        .map(|(_, clip)| (&clip.id, clip))
        .collect();
    let mut errors = Vec::new();
    for scope in super::group::scopes(project) {
        check_scope(project, scope, &everywhere, &mut errors);
    }
    errors
}

fn check_scope(
    project: &Project,
    scope: &[Track],
    everywhere: &HashMap<&ClipId, &Clip>,
    errors: &mut Vec<FollowProblem>,
) {
    let here: HashSet<&ClipId> = scope
        .iter()
        .flat_map(|track| track.clips.iter().map(|clip| &clip.id))
        .collect();
    for clip in scope.iter().flat_map(|track| &track.clips) {
        let Some(follow) = &clip.follow else {
            continue;
        };
        let (id, target) = (|| clip.id.clone(), || follow.clip.clone());
        if follow.clip == clip.id {
            errors.push(FollowProblem::Itself { clip: id() });
            continue;
        }
        let Some(arrow) = everywhere.get(&follow.clip) else {
            errors.push(FollowProblem::NoSuchClip {
                clip: id(),
                target: target(),
            });
            continue;
        };
        // A clip naming an asset that is not there is the timeline pass's to
        // report; it is not an arrow either way, and saying so twice helps
        // nobody.
        let Some(asset) = project.asset(&arrow.asset) else {
            continue;
        };
        let Some(Geometry::Arrow { from, to, .. }) = asset.shape.as_ref().map(|s| &s.geometry)
        else {
            errors.push(FollowProblem::NotAnArrow {
                clip: id(),
                target: target(),
                asset: asset.id.clone(),
            });
            continue;
        };
        if !here.contains(&follow.clip) {
            errors.push(FollowProblem::Across {
                clip: id(),
                target: target(),
            });
            continue;
        }
        for attach in [from, to].into_iter().filter_map(|end| end.attach()) {
            let follows = everywhere
                .get(&attach.clip)
                .is_some_and(|through| through.follow.is_some());
            if follows {
                errors.push(FollowProblem::Chained {
                    clip: id(),
                    arrow: target(),
                    through: attach.clip.clone(),
                });
            }
        }
    }
}
