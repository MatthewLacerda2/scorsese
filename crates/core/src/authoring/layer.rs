//! Where a video lane sits in the stack: under or over another one.
//!
//! Video tracks composite in array order, so a lane's place in its list *is*
//! its layer. A new lane goes on top ([`super::add_track`]), which is right for
//! a caption over footage and wrong for "put the logo under the captions" —
//! an edit common enough that it should not cost a whole-document write
//! (#1117).

use super::AuthorError;
use crate::project::Project;
use crate::timeline::{Track, TrackId, TrackKind};

/// Where a video lane goes: directly under, or directly over, another video
/// lane of the same stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// Drawn directly beneath this lane.
    Under(TrackId),
    /// Drawn directly above this lane.
    Over(TrackId),
}

impl Layer {
    /// The lane it is placed against.
    pub fn beside(&self) -> &TrackId {
        match self {
            Self::Under(beside) | Self::Over(beside) => beside,
        }
    }
}

/// Moves the video lane `track` to directly under or over another, and
/// changes nothing else.
///
/// A lane moves among its own stack: the timeline's lanes, or one group's.
/// Audio lanes have no layer — everything audible is summed — so naming one,
/// as the lane to move or the one to move beside, is refused.
pub fn move_track(project: &mut Project, track: &TrackId, to: &Layer) -> Result<(), AuthorError> {
    let mut proposed = project.clone();
    let lanes = stack(&mut proposed, track)?;
    let beside = to.beside();
    if beside == track {
        return Err(AuthorError::OwnLayer {
            track: track.clone(),
        });
    }
    let Some(at) = lanes.iter().position(|lane| &lane.id == beside) else {
        return Err(if project.every_track().any(|lane| &lane.id == beside) {
            AuthorError::ApartLayers {
                track: track.clone(),
                beside: beside.clone(),
            }
        } else {
            AuthorError::NoSuchTrack {
                track: beside.clone(),
            }
        });
    };
    if lanes[at].kind != TrackKind::Video {
        return Err(AuthorError::NotALayer {
            track: beside.clone(),
        });
    }
    let from = lanes
        .iter()
        .position(|lane| &lane.id == track)
        .expect("the stack was found by this lane");
    let moving = lanes.remove(from);
    let at = lanes
        .iter()
        .position(|lane| &lane.id == beside)
        .expect("only the moved lane left the stack");
    let at = match to {
        Layer::Under(_) => at,
        Layer::Over(_) => at + 1,
    };
    lanes.insert(at, moving);
    proposed.validate()?;
    *project = proposed;
    Ok(())
}

/// The list of lanes `track` is one of — the timeline's or a group's — when it
/// is a video lane.
fn stack<'a>(project: &'a mut Project, track: &TrackId) -> Result<&'a mut Vec<Track>, AuthorError> {
    let holds = |lanes: &Vec<Track>| {
        lanes
            .iter()
            .find(|lane| &lane.id == track)
            .map(|lane| lane.kind)
    };
    let lanes = project
        .stacks_mut()
        .find(|lanes| holds(lanes).is_some())
        .ok_or_else(|| AuthorError::NoSuchTrack {
            track: track.clone(),
        })?;
    match holds(lanes) {
        Some(TrackKind::Video) => Ok(lanes),
        _ => Err(AuthorError::NotALayer {
            track: track.clone(),
        }),
    }
}

#[cfg(test)]
mod tests;
