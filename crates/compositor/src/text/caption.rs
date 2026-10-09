//! How a caption looks when nobody chose: what `caption_narration` and
//! `scorsese caption` set the narration's words in.
//!
//! Here rather than in `scorsese-core` because it names a face, and which
//! names are real faces is this crate's to say. The numbers are the ones the
//! DataForce Reels ad landed on (#965): heavy Montserrat, white, rimmed in
//! black so it reads over any shot, small enough to leave the picture alone
//! on a phone.

use scorsese_core::captions::{Captioning, Chunking};
use scorsese_core::{FontChoice, PropertyPath, Rgba, TextStyle, TrackId};

use crate::path;

/// How far up from the frame's bottom edge a caption sits, as a fraction of
/// its height: the lower third, clear of the controls a phone draws over the
/// bottom of a reel.
pub const LIFT: f64 = 0.2;

/// How long, in seconds, a caption takes to arrive: about eight frames at
/// 30 fps. Quick enough to keep up with speech, slow enough to read as motion.
pub const ARRIVE: f64 = 0.27;

/// The caption style: Montserrat 800, white, a black rim.
pub fn style() -> TextStyle {
    TextStyle {
        font: FontChoice::Named("montserrat".to_owned()),
        weight: Some(800),
        size: 0.031,
        color: Rgba::WHITE,
        stroke: Some(Rgba::BLACK),
        stroke_width: 0.003,
        ..TextStyle::default()
    }
}

/// A whole run at these defaults, onto `track`, captioning every generated
/// line, on a timeline of `fps` frames a second — what a caller starts from
/// and overrides the parts it was asked about.
pub fn captioning(track: TrackId, fps: f64) -> Captioning {
    Captioning {
        track,
        narration: Vec::new(),
        chunking: Chunking::DEFAULT,
        style: style(),
        lift: LIFT,
        arrive: ((ARRIVE * fps).round() as u64).max(1),
        reveal: PropertyPath::new(path::REVEAL),
        height: PropertyPath::new(path::POSITION_Y),
    }
}
