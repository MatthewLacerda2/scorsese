//! Timeline time: an exact framerate, and positions counted in whole frames.
//!
//! A project is authored against a **timeline framerate** — the grid every
//! clip and keyframe sits on — stored as an exact rational ([`Fps`]). Times
//! are integer frame counts on that grid ([`Frames`]), never float seconds.
//!
//! That is two things kept apart, not one thing bound too early:
//!
//! - the **timeline** framerate, decided once when the project is created,
//!   which answers *what is on screen when*; and
//! - the **output** framerate, a render setting chosen per render, which a
//!   render conforms to from the timeline grid.
//!
//! Having the first does not bake delivery into the project, and losing it
//! costs exactness: float seconds make a cut ambiguous (if a clip ends at
//! `1.0333333` and the next starts there, neither owns the frame) and drift
//! on NTSC rates, which are rationals — 29.97 is exactly `30000/1001` and
//! nothing else.
//!
//! Seconds still exist, as `f64`, at the two edges where wall-clock is the
//! right unit: showing a time to a human, and handing one to ffmpeg. They are
//! a conversion ([`Fps::seconds`]), never a stored timeline value.

mod fps;
mod frames;

pub use fps::{Fps, FpsError, FpsParseError};
pub use frames::Frames;
