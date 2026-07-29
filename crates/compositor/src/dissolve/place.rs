//! Making room for the crossover: where the incoming clip ends up.
//!
//! Tracks are ordered and a later one draws over an earlier one, so putting
//! the incoming clip *above* the outgoing one is what makes it the shot that
//! arrives. Two clips overlapping across tracks is ordinary compositing; two
//! overlapping on one track is what the format forbids, and that is the whole
//! reason this module exists.

use scorsese_core::{Clip, ClipId, Frames, Project, Track, TrackId, TrackKind};

/// What a dissolve had to rearrange, so a caller can say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    /// The track the incoming clip now sits on.
    pub track: String,
    /// Whether that track had to be made for it.
    pub track_created: bool,
    /// How far back the incoming clip was pulled, which is the dissolve's
    /// length — stated because it is the edit a caller did not ask for in so
    /// many words.
    pub pulled_back: u64,
}

/// Why a dissolve could not be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DissolveError {
    /// No clip in the project has this id.
    #[error("no clip `{clip}` in this project")]
    NoSuchClip {
        /// The id as asked for.
        clip: String,
    },
    /// A dissolve is a picture operation; sound crossfades by being mixed.
    #[error("clip `{clip}` is on an audio track, and a dissolve is a picture operation")]
    NotPicture {
        /// The clip on the wrong kind of track.
        clip: String,
    },
    /// The two clips do not meet, so there is no crossover to put anywhere.
    #[error(
        "clip `{from}` ends at frame {from_ends} and `{to}` starts at frame {to_starts}, \
         so there is no cut between them to dissolve across"
    )]
    DoNotMeet {
        /// The outgoing clip.
        from: String,
        /// The frame just past its last.
        from_ends: u64,
        /// The incoming clip.
        to: String,
        /// Where it starts.
        to_starts: u64,
    },
    /// One of the clips is shorter than the dissolve asked for.
    #[error("clip `{clip}` is {available} frames long, so it cannot give {asked} to a dissolve")]
    TooShort {
        /// The clip that cannot give the frames.
        clip: String,
        /// How long it is.
        available: u64,
        /// How long the dissolve asked to be.
        asked: u64,
    },
    /// The dissolve would put the incoming clip over one already there.
    #[error(
        "pulling `{clip}` back {by} frames would overlap `{onto}` on every track above it, \
         and clips on one track may not overlap"
    )]
    NoRoom {
        /// The incoming clip.
        clip: String,
        /// How far it needed to move.
        by: u64,
        /// What is in the way.
        onto: String,
    },
}

/// Where a clip is: which track holds it, and where in that track's list.
#[derive(Debug, Clone, Copy)]
struct At {
    track: usize,
    clip: usize,
}

/// Pulls the incoming clip back over the outgoing one, on a track above it.
///
/// # Errors
///
/// See [`DissolveError`]. Nothing is changed unless every check passes, so a
/// refused dissolve leaves the project exactly as it was.
pub(super) fn overlap(
    project: &mut Project,
    from: &ClipId,
    to: &ClipId,
    duration: Frames,
) -> Result<Placed, DissolveError> {
    let (outgoing, incoming) = (locate(project, from)?, locate(project, to)?);
    picture(project, outgoing, from)?;
    picture(project, incoming, to)?;
    meeting(project, outgoing, incoming, from, to)?;
    long_enough(project, outgoing, from, duration)?;
    long_enough(project, incoming, to, duration)?;

    // Everything above is a check. From here the project is edited, and it is
    // written so that the first mutation happens only once nothing can refuse.
    let mut clip = project.tracks[incoming.track].clips.remove(incoming.clip);
    clip.start = Frames(clip.start.get().saturating_sub(duration.get()));

    let (track, created) = home(project, outgoing.track, &clip)?;
    project.tracks[track].clips.push(clip);
    project.tracks[track]
        .clips
        .sort_by_key(|clip| clip.start.get());

    Ok(Placed {
        track: project.tracks[track].id.to_string(),
        track_created: created,
        pulled_back: duration.get(),
    })
}

/// A track above `below` with room for `clip`, making one if none has room.
fn home(project: &mut Project, below: usize, clip: &Clip) -> Result<(usize, bool), DissolveError> {
    let above = below + 1;
    for (index, track) in project.tracks.iter().enumerate().skip(above) {
        if track.kind == TrackKind::Video && !track.clips.iter().any(|other| clip.overlaps(other)) {
            return Ok((index, false));
        }
    }
    // Nothing above has room, so the dissolve gets a track of its own — put
    // directly over the outgoing clip rather than at the top, so an unrelated
    // overlay stays above both.
    let id = TrackId::new(free_id(project));
    project.tracks.insert(
        above,
        Track {
            id,
            kind: TrackKind::Video,
            name: None,
            clips: Vec::new(),
        },
    );
    Ok((above, true))
}

/// A track id nothing is using.
fn free_id(project: &Project) -> String {
    (1..)
        .map(|n| format!("v-dissolve-{n}"))
        .find(|id| !project.tracks.iter().any(|track| track.id.as_str() == id))
        .unwrap_or_else(|| "v-dissolve".to_owned())
}

/// Where this clip is, or which id was not found.
fn locate(project: &Project, id: &ClipId) -> Result<At, DissolveError> {
    project
        .tracks
        .iter()
        .enumerate()
        .find_map(|(track, holding)| {
            holding
                .clips
                .iter()
                .position(|clip| &clip.id == id)
                .map(|clip| At { track, clip })
        })
        .ok_or_else(|| DissolveError::NoSuchClip {
            clip: id.to_string(),
        })
}

/// Refuses a clip that is not on a video track.
fn picture(project: &Project, at: At, id: &ClipId) -> Result<(), DissolveError> {
    if project.tracks[at.track].kind == TrackKind::Video {
        return Ok(());
    }
    Err(DissolveError::NotPicture {
        clip: id.to_string(),
    })
}

/// Refuses two clips with no cut between them.
fn meeting(
    project: &Project,
    outgoing: At,
    incoming: At,
    from: &ClipId,
    to: &ClipId,
) -> Result<(), DissolveError> {
    let ends = project.tracks[outgoing.track].clips[outgoing.clip].end();
    let starts = project.tracks[incoming.track].clips[incoming.clip].start;
    if ends == starts {
        return Ok(());
    }
    Err(DissolveError::DoNotMeet {
        from: from.to_string(),
        from_ends: ends.get(),
        to: to.to_string(),
        to_starts: starts.get(),
    })
}

/// Refuses a clip too short to give the dissolve its frames.
fn long_enough(
    project: &Project,
    at: At,
    id: &ClipId,
    duration: Frames,
) -> Result<(), DissolveError> {
    let available = project.tracks[at.track].clips[at.clip].duration;
    if available >= duration && duration.get() > 0 {
        return Ok(());
    }
    Err(DissolveError::TooShort {
        clip: id.to_string(),
        available: available.get(),
        asked: duration.get(),
    })
}
