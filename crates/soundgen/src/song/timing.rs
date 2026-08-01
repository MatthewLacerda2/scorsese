//! Making a song the length the picture needs.
//!
//! A song's natural length is whatever its notes add up to, plus however long
//! the last one takes to stop ringing. That is the right answer for a game,
//! where a loop is a loop and nothing is waiting on it. It is the wrong answer
//! for video, where the music has a hole to fill: forty-three seconds between
//! two cuts.
//!
//! Everything here is optional, and absent means the song is as long as it is.

use serde::{Deserialize, Serialize};

/// How far a tempo may be moved to make a song fit, as a fraction either way.
///
/// A bed at 40% speed is not a bed, it is a mistake — so `stretch` refuses
/// past this rather than delivering something nobody would use. A quarter is
/// already a large musical change; the point of the bound is that the refusal
/// says what tempo it would have needed, which is enough to decide what to do
/// instead.
pub(crate) const MAX_STRETCH: f32 = 0.25;

/// A length the song must come out at.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fit {
    /// The target length in seconds — the hole in the cut.
    pub seconds: f32,
    /// How to get there.
    #[serde(default)]
    pub mode: FitMode,
}

/// What a song does when it is not already the length it has to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    /// Repeat the arrangement until the target is reached, cutting
    /// mid-arrangement if that is where the time runs out.
    ///
    /// The default, because it is what a bed under dialogue wants: the seam is
    /// inaudible under speech, and the music keeps its own tempo.
    #[default]
    Loop,
    /// Move the tempo so a whole number of arrangement passes lands exactly on
    /// the target.
    ///
    /// Keeps the music intact and changes how fast it is played instead.
    /// Refuses rather than distorts past a quarter either way.
    Stretch,
    /// Play through once and pad with silence to the target.
    ///
    /// Honest, and the right answer for a sting: a sound that happens once and
    /// then is over does not want to be looped into a texture.
    Once,
}

/// Level moves applied to the finished piece.
///
/// In the recipe rather than left to clip keyframes because a fade that
/// belongs to the *music* — a piece that ends by resolving and receding — is a
/// property of the piece, and should survive being moved elsewhere in the
/// timeline or used in another project. Keyframes on the clip stay the right
/// tool for ducking *this* use of it under *this* voice-over.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fade {
    /// Seconds from silence to full level at the start.
    #[serde(default)]
    pub in_seconds: f32,
    /// Seconds from full level to silence at the end.
    #[serde(default)]
    pub out_seconds: f32,
}

impl Fade {
    /// True when neither end does anything, so the whole pass can be skipped.
    pub(crate) fn is_silent_about_everything(self) -> bool {
        self.in_seconds <= 0.0 && self.out_seconds <= 0.0
    }
}

/// What happens after the last beat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tail {
    /// Let it ring: the buffer grows to fit the last note's release and any fx
    /// tail, so the piece ends by stopping rather than by being stopped.
    #[default]
    Ring,
    /// End exactly on the arrangement's last beat, with the tail faded into
    /// it. What a caller wants when the music has to butt against something.
    Exact,
}
