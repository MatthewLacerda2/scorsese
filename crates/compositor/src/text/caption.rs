//! How a caption looks when nobody chose: what `caption_narration` and
//! `scorsese caption` set the narration's words in.
//!
//! Here rather than in `scorsese-core` because it names a face, and which
//! names are real faces is this crate's to say. The numbers are the ones the
//! DataForce Reels ad landed on (#965): heavy Montserrat, white, rimmed in
//! black so it reads over any shot, small enough to leave the picture alone
//! on a phone.

use std::str::FromStr;

use scorsese_core::captions::{Captioning, Chunking};
use scorsese_core::{ColorError, FontChoice, PropertyPath, Rgba, TextStyle, TrackId};

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
        // The mixer's to name: the caller that renders sound sets it.
        volume: None,
    }
}

/// The rim outside each letter as asked for: a colour, or `none` for no rim.
///
/// One spelling for both surfaces — `--stroke` on the command line and
/// `stroke` on `caption_narration` read it through the same parser (#1288).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stroke(pub Option<Rgba>);

impl FromStr for Stroke {
    type Err = ColorError;

    /// `none`, or anything [`Rgba`] reads.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.trim().eq_ignore_ascii_case("none") {
            return Ok(Self(None));
        }
        text.trim().parse().map(|colour| Self(Some(colour)))
    }
}

/// What a caller was asked about a caption run, already parsed. Anything left
/// `None` keeps the default [`captioning`] starts from.
#[derive(Debug, Clone, Default)]
pub struct Choices {
    /// The audio tracks whose lines are captioned; empty is every one.
    pub narration: Vec<TrackId>,
    /// The most characters one caption holds.
    pub max_chars: Option<usize>,
    /// The face. A file is whatever weight it is, so naming one drops the
    /// default weight.
    pub font: Option<FontChoice>,
    /// How heavy, 1 to 1000.
    pub weight: Option<u16>,
    /// How big, as a fraction of the frame's height.
    pub size: Option<f64>,
    /// The words' colour.
    pub color: Option<Rgba>,
    /// The rim outside each letter.
    pub stroke: Option<Stroke>,
    /// How far up from the frame's bottom edge, as a fraction of its height.
    pub lift: Option<f64>,
}

impl Choices {
    /// The whole run onto `track` on a timeline of `fps`: [`captioning`]'s
    /// defaults with these choices laid over them — the one function the
    /// command line and the MCP server both build a caption run with.
    pub fn captioning(self, track: TrackId, fps: f64) -> Captioning {
        let mut asked = captioning(track, fps);
        asked.narration = self.narration;
        asked.chunking.max_chars = self.max_chars.unwrap_or(asked.chunking.max_chars);
        asked.lift = self.lift.unwrap_or(asked.lift);
        let style = &mut asked.style;
        if let Some(font) = self.font {
            // A file is whatever weight it is, and says so itself; naming one
            // beside a static file is refused.
            if font.file().is_some() {
                style.weight = None;
            }
            style.font = font;
        }
        style.weight = self.weight.or(style.weight);
        style.size = self.size.unwrap_or(style.size);
        style.color = self.color.unwrap_or(style.color);
        if let Some(Stroke(stroke)) = self.stroke {
            style.stroke = stroke;
        }
        asked
    }
}
