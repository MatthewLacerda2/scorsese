//! Cutting a narrated video to its voice (#1008).
//!
//! In a narrated video the voice decides the pace and the picture follows it.
//! Each **scene** pairs one narration clip with the visual clip or clips that
//! illustrate it, and the cut is laid out from what the lines actually say:
//!
//! ```text
//!   scene 1                         scene 2
//!   |lead|-- line 1 --| gap |        |lead|---- line 2 ----| gap |
//!   [====== visuals 1 ===========]<- overlap
//!                             [====== visuals 2 ================]
//!                             ^ the next scene begins `gap` after line 1's last word
//! ```
//!
//! - each line starts its scene's **lead-in** after the scene begins;
//! - each scene ends a **gap** after its line's last word — measured from the
//!   word timings kept beside a generated line ([`crate::words`]), or from the
//!   end of the audio when there are none, and [`Laid::measured`] says which;
//! - the next scene begins right there, and with an **overlap** the outgoing
//!   visuals run on that much longer, so an exit and an entrance can play
//!   together.
//!
//! **Scenes are named, never inferred** from track order: a wrong guess
//! silently re-times a whole cut, so the caller lists them in order.
//!
//! **What comes out is ordinary document content** — clip starts and
//! durations, and the track a clip sits on — the bargain [`crate::dip`] and
//! [`crate::captions`] keep. Two visuals overlapping cannot share a track, so a
//! clip with no room left on its own goes to the next track of its kind that
//! has room, or one made for it directly above; [`Voiced::rearranged`] names
//! each. That is the alternating pair of scene tracks a hand-built overlap
//! uses, arrived at rather than declared.
//!
//! **What follows a scene moves with it.** A clip listed as one of a scene's
//! riders (its sound effects, an overlay) keeps its offset from the scene's
//! start. A clip not named at all stays exactly where it is, and
//! [`Voiced::crossed`] lists those whose place in the cut changed under them.
//!
//! **All or nothing.** The layout is worked out on a copy and becomes the
//! document only once [`Project::validate`] accepts it, so a media-backed
//! visual that would now outrun its source is refused by name and nothing
//! moves. A page, a still or a title has no source length and simply
//! stretches; a page reads its clip's length (#810), so its exit lands on the
//! new end.

mod lay;
mod report;
mod room;

use std::path::Path;

use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{ClipId, TrackId};
use crate::validate::ValidationErrors;

/// One scene: a line and what is seen while it is said.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// The narration clip — the line.
    pub line: ClipId,
    /// The visual clips that illustrate it. Each one runs the whole scene.
    pub visuals: Vec<ClipId>,
    /// Clips that keep their offset from the scene's start — its sound
    /// effects, an overlay that arrives part-way.
    pub riders: Vec<ClipId>,
    /// This scene's lead-in, in seconds, in place of [`Voicing::lead_in`].
    pub lead_in: Option<f64>,
}

/// One call: the scenes in order, and the timing between them.
#[derive(Debug, Clone, PartialEq)]
pub struct Voicing {
    /// The scenes, in the order they play.
    pub scenes: Vec<Scene>,
    /// Seconds from a scene's start to its line's start.
    pub lead_in: f64,
    /// Seconds from a line's last word to the end of its scene.
    pub gap: f64,
    /// Seconds a scene's visuals run on past its end, under the next one.
    pub overlap: f64,
    /// Where the first scene begins, in seconds. `None` keeps it where its
    /// earliest visual already starts.
    pub from: Option<f64>,
}

/// Where a scene's end was measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measured {
    /// The line's last spoken word, from its word timings.
    LastWord,
    /// The end of the line's clip: it has no word timings — imported speech,
    /// or a line not generated with them.
    EndOfAudio,
}

/// One scene, as laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct Laid {
    /// The scene's line.
    pub line: ClipId,
    /// Where the scene begins.
    pub start: Frames,
    /// Where it ends: `gap` after the line's last word.
    pub end: Frames,
    /// What that end was measured from.
    pub measured: Measured,
}

/// What a cut did, once the document accepted it.
#[derive(Debug, Clone, PartialEq)]
pub struct Voiced {
    /// Every scene, in order.
    pub scenes: Vec<Laid>,
    /// Where the scenes used to end — the last visual's end, before.
    pub was: Frames,
    /// Clips that moved to another track to make room, with the track and
    /// whether it was made for them.
    pub rearranged: Vec<(ClipId, TrackId, bool)>,
    /// Clips the call did not name whose place in the cut changed: the scenes
    /// they overlap are not the ones they overlapped before.
    pub crossed: Vec<ClipId>,
    /// Visuals carrying keyframes past their new end — a fade-out written for
    /// the old length, most often.
    pub keyed_past_end: Vec<ClipId>,
    /// What else reads the old positions, and should be run again.
    pub follow_ups: Vec<&'static str>,
}

/// Why a cut did not happen. Nothing is ever partly applied.
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    /// The call named no scene.
    #[error("name at least one scene")]
    NoScenes,
    /// A scene with no visual has nothing to stretch.
    #[error("the scene of `{0}` names no visual clip")]
    NoVisual(ClipId),
    /// A lead-in, gap or overlap that is not a non-negative number.
    #[error("`{0}` must be a number of seconds, 0 or more")]
    Seconds(&'static str),
    /// No clip in the project has this id.
    #[error("no clip `{0}` in this project")]
    NoSuchClip(ClipId),
    /// One clip named twice, so it would be laid out twice.
    #[error("`{0}` is named more than once")]
    Twice(ClipId),
    /// A line that is not on an audio track.
    #[error("`{0}` is a line, so it must be on an audio track")]
    NotALine(ClipId),
    /// A visual that is not on a video track.
    #[error("`{0}` is a visual, so it must be on a video track")]
    NotAVisual(ClipId),
    /// A rider that would move before the first frame of the timeline.
    #[error("`{0}` would start before the beginning of the timeline")]
    BeforeTheStart(ClipId),
    /// The result was not a document that loads — a media-backed visual run
    /// past the end of its source, most often.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Lays `voicing`'s scenes out from their lines, in `project`, whose folder is
/// `root` — where each line's word timings are read from.
///
/// # Errors
///
/// See [`VoiceError`]. A refused cut leaves `project` exactly as it was.
pub fn cut_to_voice(
    project: &mut Project,
    root: &Path,
    voicing: &Voicing,
) -> Result<Voiced, VoiceError> {
    lay::check(project, voicing)?;
    let plan = lay::plan(project, root, voicing)?;
    let mut proposed = project.clone();
    let rearranged = room::apply(&mut proposed, &plan.moves);
    proposed.validate()?;
    let voiced = Voiced {
        crossed: report::crossed(project, &proposed, voicing, &plan),
        keyed_past_end: report::keyed_past_end(&proposed, voicing),
        follow_ups: report::follow_ups(&proposed),
        scenes: plan.laid,
        was: plan.was,
        rearranged,
    };
    *project = proposed;
    Ok(voiced)
}

#[cfg(test)]
mod tests;
