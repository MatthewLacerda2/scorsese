//! Applying a dip across a whole project.

use crate::keyframe::PropertyPath;
use crate::project::Project;
use crate::timeline::{Clip, TrackId, TrackKind};

use super::{Dip, TOOL, covering, is_coverable, plan};

/// What one run of ducking did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ducked {
    /// Clips that came away with a dip on them.
    pub dipped: Vec<String>,
    /// Clips nothing covered, which therefore keep no generated track — and
    /// lose one they had, if a previous run put it there and the narration has
    /// since moved away.
    pub untouched: Vec<String>,
}

impl Ducked {
    /// True when this run changed nothing anywhere.
    pub fn is_empty(&self) -> bool {
        self.dipped.is_empty() && self.untouched.is_empty()
    }
}

/// Which track is being ducked, and what is allowed to duck it.
#[derive(Debug, Clone)]
pub struct Under {
    /// The track whose clips get dipped.
    pub music: TrackId,
    /// The tracks whose clips do the covering. Empty means *every other audio
    /// track*, which is the common arrangement and a bad thing to have to
    /// spell out.
    pub narration: Vec<TrackId>,
}

impl Under {
    /// True when `track` counts as narration for this run.
    fn covers(&self, id: &TrackId, kind: TrackKind) -> bool {
        if id == &self.music {
            return false;
        }
        if self.narration.is_empty() {
            return kind == TrackKind::Audio;
        }
        self.narration.contains(id)
    }
}

/// Writes a dip onto every clip of the music track, under everything on the
/// narration tracks.
///
/// Idempotent by construction: each run replaces only the tracks signed
/// [`TOOL`], so running it twice leaves one dip and running it after a hand
/// edit leaves the hand edit alone. A clip nothing covers has its generated
/// track **removed** rather than left behind — if the narration moved away,
/// the old dip is a ramp the project no longer means.
///
/// `Ok(None)` when there is no such track, which is the one failure worth
/// distinguishing: everything else is a legitimate outcome, including "nothing
/// covers anything".
pub fn duck_track(
    project: &mut Project,
    under: &Under,
    property: &PropertyPath,
    dip: Dip,
) -> Option<Ducked> {
    // Collected before the mutable borrow, and cloned because a covering clip
    // may live on any track — including, in principle, one being ducked in a
    // later call.
    let covers: Vec<Clip> = project
        .tracks
        .iter()
        .filter(|track| under.covers(&track.id, track.kind))
        .flat_map(|track| track.clips.iter().cloned())
        .collect();

    let music = project
        .tracks
        .iter_mut()
        .find(|track| track.id == under.music)?;

    let mut report = Ducked::default();
    for clip in &mut music.clips {
        let spans = covering(clip, &covers);
        match plan(clip, &spans, property.clone(), dip) {
            Some(track) => {
                clip.replace_generated(TOOL, vec![track]);
                report.dipped.push(clip.id.as_str().to_owned());
            }
            None => {
                clip.replace_generated(TOOL, vec![]);
                report.untouched.push(clip.id.as_str().to_owned());
            }
        }
    }
    Some(report)
}

/// The audio tracks a project has, in document order — what a caller offers
/// when it has to ask which one is the music.
pub fn audio_tracks(project: &Project) -> impl Iterator<Item = &TrackId> {
    project
        .tracks
        .iter()
        .filter(|track| is_coverable(track))
        .map(|track| &track.id)
}
