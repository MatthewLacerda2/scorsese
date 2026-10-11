//! Applying a dip across a whole project.

use std::ops::Range;

use crate::keyframe::PropertyPath;
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{ClipId, TrackId, TrackKind};

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

/// Why a run of ducking changed nothing at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DuckError {
    /// The track to duck is not in the project.
    #[error("no track `{0}` in this project")]
    NoMusic(String),
    /// Something named as narration is not in the project: a typo'd id would
    /// otherwise duck under nothing and say it had succeeded.
    #[error("nothing named {} in this project — no track or clip by that id", quoted(.0))]
    Unknown(Vec<String>),
}

fn quoted(ids: &[String]) -> String {
    let ids: Vec<String> = ids.iter().map(|id| format!("`{id}`")).collect();
    ids.join(", ")
}

/// Which track is being ducked, and what is allowed to duck it.
///
/// The three ways of naming narration add up: a stretch counts as covered when
/// any one of them covers it. Naming **none** of them means every other audio
/// track, which is the common arrangement and a bad thing to have to spell
/// out.
#[derive(Debug, Clone)]
pub struct Under {
    /// The track whose clips get dipped.
    pub music: TrackId,
    /// Tracks whose clips do the covering, of any kind.
    pub narration: Vec<TrackId>,
    /// Single clips that do the covering, wherever they sit — a line spoken
    /// inside a video shot, say, on a track whose other shots are not speech.
    pub clips: Vec<ClipId>,
    /// Stretches of the timeline that count as narration with no clip at all,
    /// in absolute frames: the words a page reads, or captions with no audio
    /// of their own. Unlike a clip, a range does not follow an edit.
    pub ranges: Vec<Range<Frames>>,
}

impl Under {
    /// Ducks `music` under every other audio track — what naming nothing means.
    pub fn new(music: TrackId) -> Self {
        Self {
            music,
            narration: Vec::new(),
            clips: Vec::new(),
            ranges: Vec::new(),
        }
    }

    fn names_nothing(&self) -> bool {
        self.narration.is_empty() && self.clips.is_empty() && self.ranges.is_empty()
    }

    /// Every stretch of the timeline this run ducks under, absolute.
    ///
    /// A clip on the music track itself never counts: music does not step
    /// aside for itself.
    fn stretches(&self, project: &Project) -> Vec<Range<Frames>> {
        let everything_else = self.names_nothing();
        let mut stretches: Vec<Range<Frames>> = project
            .tracks
            .iter()
            .filter(|track| track.id != self.music)
            .flat_map(|track| {
                let whole = if everything_else {
                    track.kind == TrackKind::Audio
                } else {
                    self.narration.contains(&track.id)
                };
                track
                    .clips
                    .iter()
                    .filter(move |clip| whole || self.clips.contains(&clip.id))
            })
            .map(|clip| clip.start..clip.end())
            .collect();
        stretches.extend(self.ranges.iter().cloned());
        stretches
    }

    /// The tracks and clips this names that the project does not have.
    fn unknown(&self, project: &Project) -> Vec<String> {
        let tracks = self
            .narration
            .iter()
            .filter(|id| !project.tracks.iter().any(|track| &track.id == *id))
            .map(|id| id.as_str().to_owned());
        let clips = self
            .clips
            .iter()
            .filter(|id| {
                !project
                    .tracks
                    .iter()
                    .any(|track| track.clips.iter().any(|clip| &clip.id == *id))
            })
            .map(|id| id.as_str().to_owned());
        tracks.chain(clips).collect()
    }
}

/// Writes a dip onto every clip of the music track, under everything
/// [`Under`] names.
///
/// Idempotent by construction: each run replaces only the tracks signed
/// [`TOOL`], so running it twice leaves one dip and running it after a hand
/// edit leaves the hand edit alone. A clip nothing covers has its generated
/// track **removed** rather than left behind — if the narration moved away,
/// the old dip is a ramp the project no longer means.
///
/// Refuses, changing nothing, when the music track or anything named as
/// narration is not there. Everything else is a legitimate outcome, including
/// "nothing covers anything".
pub fn duck_track(
    project: &mut Project,
    under: &Under,
    property: &PropertyPath,
    dip: Dip,
) -> Result<Ducked, DuckError> {
    let unknown = under.unknown(project);
    if !unknown.is_empty() {
        return Err(DuckError::Unknown(unknown));
    }
    // Collected before the mutable borrow: a covering clip may live on any
    // track, including one being ducked in a later call.
    let covers = under.stretches(project);
    let music = project
        .tracks
        .iter_mut()
        .find(|track| track.id == under.music)
        .ok_or_else(|| DuckError::NoMusic(under.music.as_str().to_owned()))?;

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
    Ok(report)
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
