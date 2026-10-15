//! Lowering one clip's property while other clips are playing.
//!
//! This is the mechanism behind **auto-ducking** — music stepping aside for
//! narration — but it is written without ever naming `volume`, and that is
//! deliberate.
//!
//! `scorsese-core` defines property *types*, never property *values*, and the
//! meaning of `volume` belongs to the mixer that implements it. So a caller
//! hands in the [`PropertyPath`](crate::PropertyPath) to dip and the two levels to dip between, and
//! this module does the arithmetic: which spans of a clip are covered, which of
//! those are close enough to be one dip rather than two, and where the ramps
//! land.
//!
//! What comes out is an ordinary [`KeyframeTrack`](crate::KeyframeTrack) — the same thing a person
//! would have placed by hand, evaluated by the same [`KeyframeTrack::value_at`](crate::KeyframeTrack::value_at)
//! that evaluates a fade. There is no ducking stage anywhere in the mixer, and
//! there should never be one.
//!
//! ```text
//!      ────────╮                          ╭────────   over (between lines)
//!               ╲                        ╱
//!                ╰──────────────────────╯             under
//!        attack  │      covered span    │  release
//! ```
//!
//! What counts as covering is up to the caller ([`Under`]): whole tracks of
//! any kind, single clips wherever they sit, or bare stretches of the timeline
//! with no clip at all — the narration a page reads from `scorsese.words`, or a
//! line spoken inside a video shot.

mod apply;
mod plan;
mod spans;

use crate::time::Frames;

pub use apply::{DuckError, Ducked, Under, duck_track};
pub(crate) use plan::plan;
pub(crate) use spans::{Span, covering, merge};

/// Who signs the keyframe tracks this module produces.
///
/// A constant rather than a caller's choice, because the whole value of the
/// signature is that the *next* run recognises it — a name that varies is a
/// name that never matches.
pub const TOOL: &str = "duck";

/// How far the property dips, and how long it takes to get there and back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dip {
    /// The value held while something is covering. For volume, a multiplier:
    /// `0.25` is a quarter as loud.
    pub under: f64,
    /// The value everywhere else — what the clip reads between lines, before
    /// the first and after the last. For volume, the level the music rests at.
    pub over: f64,
    /// How long the fall takes, in frames. The dip is fully down *by* the
    /// moment the covering clip starts, not after it.
    pub attack: Frames,
    /// How long the climb back takes, in frames.
    pub release: Frames,
    /// The longest pause that stays ducked: a pause this long or shorter is
    /// held under as part of one dip, and a longer one climbs back to
    /// [`Dip::over`] before the next line. `None` is one recovery plus one fall
    /// ([`Dip::gap_worth_keeping`]).
    ///
    /// Shorter than the two ramps, a pause has no room for both, so it shares
    /// itself between them in proportion and the level touches `over` once on
    /// its way back down. A breath rather than a plateau.
    pub min_gap: Option<Frames>,
}

impl Dip {
    /// How far a dip goes when nobody says: a quarter of the clip's own level.
    ///
    /// This and the two times below are the one place the ducking defaults
    /// live. The CLI's `duck` and MCP's `duck_music` both read them, so tuning
    /// the duck here tunes it everywhere.
    pub const UNDER: f64 = 0.25;
    /// Seconds the fall takes when nobody says.
    pub const ATTACK_SECONDS: f64 = 0.3;
    /// Seconds the recovery takes when nobody says — twice the fall.
    pub const RELEASE_SECONDS: f64 = 0.6;
    /// Where the level rests between lines when nobody says: the clip's own
    /// level, untouched.
    pub const BETWEEN: f64 = 1.0;

    /// A dip to `under` from [`Dip::BETWEEN`], its ramps given in seconds and
    /// put on a grid of `fps` frames per second, merging pauses by the default
    /// rule.
    ///
    /// A ramp asked for at all is at least one frame long: rounded to zero it
    /// would be a switch, not a ramp. Zero or less is no ramp — which is still
    /// drawn over one frame, the shortest step two keyframes can make.
    pub fn from_seconds(under: f64, attack: f64, release: f64, fps: f64) -> Self {
        let ramp = |seconds: f64| Frames::span(seconds, fps);
        Self {
            under,
            over: Self::BETWEEN,
            attack: ramp(attack),
            release: ramp(release),
            min_gap: None,
        }
    }

    /// This dip with its [`Dip::min_gap`] given in seconds on a grid of `fps`
    /// frames per second. Zero, a negative or NaN keeps every pause the grid
    /// has room to breathe in.
    #[must_use]
    pub fn with_min_gap_seconds(self, seconds: f64, fps: f64) -> Self {
        // A float cast to an integer saturates, and NaN lands on zero, so
        // nonsense becomes "keep every pause" without a branch for it.
        let frames = Frames((seconds * fps).round() as u64);
        Self {
            min_gap: Some(frames),
            ..self
        }
    }

    /// A quarter-level dip with a fast fall and a slower recovery, on a grid
    /// of `fps` frames per second: [`Dip::UNDER`], [`Dip::ATTACK_SECONDS`]
    /// and [`Dip::RELEASE_SECONDS`].
    ///
    /// The asymmetry is the point and it is how every ducking compressor ever
    /// built behaves: getting out of the way late is audible as a word being
    /// stepped on, and coming back early is audible as a lurch. So the fall is
    /// quick and the recovery is unhurried.
    pub fn default_for(fps: f64) -> Self {
        Self::from_seconds(
            Self::UNDER,
            Self::ATTACK_SECONDS,
            Self::RELEASE_SECONDS,
            fps,
        )
    }

    /// The shortest gap between two covered spans worth treating as a gap.
    ///
    /// Two lines of narration a beat apart should be one dip, not two: coming
    /// all the way back up and immediately going down again is pumping, and it
    /// draws more attention than the thing it was avoiding. So unless
    /// [`Dip::min_gap`] says otherwise, anything no longer than a full
    /// recovery and fall is merged.
    ///
    /// Never less than one frame: a one-frame pause leaves no frame between
    /// the end of one dip and the start of the next for the level to rise on.
    pub fn gap_worth_keeping(self) -> Frames {
        self.min_gap
            .unwrap_or(self.release + self.attack)
            .max(Frames(1))
    }
}
