//! Less of a song: a stretch of it, some of its tracks, or both.
//!
//! An excerpt is **not a new sound**. It is the same renderer asked for less
//! of the same piece, and the promise it makes is exact: the samples a window
//! hands back are the samples a whole render would have put there, bit for
//! bit. That promise is the whole feature — a mix decision taken on eight bars
//! is only worth taking if those eight bars are the ones the file will have.
//!
//! ## Which clock a window counts on
//!
//! **The rendered piece, not the written arrangement.** `fit` can change both
//! the tempo and the number of passes, and `automation` already reads its
//! beats off the result rather than off the page — so a window that counted
//! the arrangement would be the one coordinate in the crate that disagreed
//! with the others. Under a `loop` fit the two genuinely differ: beat 40 of a
//! sixteen-beat arrangement looped four times is in the third pass, and that
//! is what a window asking for beat 40 gets, because it is what is audible
//! there.
//!
//! Seconds count on the same clock, and are the same window said the other
//! way: at the tempo the piece is actually rendered at.
//!
//! ## Why a window still renders what comes before it
//!
//! Everything in the signal path is causal but for two stages, and both of
//! them are the reason a window cannot simply start where it starts:
//!
//! - a note before the window rings into it, and its bus and the song's chain
//!   decay across the seam;
//! - the limiter and any compressor **look ahead**, so a peak just past the
//!   window pulls the gain down inside it.
//!
//! So a window renders the piece from the top and keeps the stretch asked
//! for, and it renders a [`guard`] past the end for the lookahead to see.
//! What it saves is every note after that guard, which for the case this
//! exists to serve — the first bars of a long bed — is nearly all of them.
//! A window at the very end of a piece saves nothing, and that is honest
//! rather than a gap: you cannot know what the last bar sounds like without
//! playing up to it.

use std::fmt;
use std::str::FromStr;

use super::Song;
use super::shape::SEAM;
use crate::core::RATE;
use crate::error::SynthError;
use crate::fx;

/// The extra rendering a window does past its own end so that the stages
/// which look ahead see what they would have seen, expressed in samples.
///
/// Small change against the reconstruction the limiter measures peaks with,
/// which reads a handful of samples either side of the frame it is asked
/// about. Rounded up generously — the cost of a few spare milliseconds is
/// nothing and the cost of being one sample short is a promise broken.
const RECONSTRUCTION: usize = 64;

/// What less of a song to render: a stretch of it, a set of its tracks, or
/// both. The default is the whole piece, which is what every render that does
/// not ask for less passes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Excerpt {
    /// Which stretch of the rendered piece to keep. `None` is all of it.
    pub window: Option<Window>,
    /// Which tracks reach the mix, by name. Empty is all of them.
    ///
    /// A track left out is still **played** when something is keyed off it, so
    /// a solo shows the duck the mix has rather than one it does not. It is
    /// simply not heard.
    pub only: Vec<String>,
}

impl Excerpt {
    /// True when this asks for the piece exactly as a whole render makes it.
    pub fn is_whole(&self) -> bool {
        self.window.is_none() && self.only.is_empty()
    }

    /// A stretch of the piece, and every track in it.
    pub fn of(window: Window) -> Self {
        Self {
            window: Some(window),
            only: Vec::new(),
        }
    }

    /// These tracks, across the whole piece.
    pub fn only(tracks: Vec<String>) -> Self {
        Self {
            window: None,
            only: tracks,
        }
    }
}

/// A stretch of the rendered piece, counted in beats or in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    span: Span,
    unit: Unit,
}

/// What a window's two numbers count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Beats of the rendered piece — the unit the document itself is written
    /// in, so a window lines up with a pattern length and an automation point
    /// without anybody converting anything.
    Beats,
    /// Seconds of the rendered piece, for a question the picture asked rather
    /// than the music.
    Seconds,
}

impl Window {
    /// A window in beats of the rendered piece.
    pub const fn beats(span: Span) -> Self {
        Self {
            span,
            unit: Unit::Beats,
        }
    }

    /// A window in seconds of the rendered piece.
    pub const fn seconds(span: Span) -> Self {
        Self {
            span,
            unit: Unit::Seconds,
        }
    }

    /// The first sample-frame kept, and the one just past the last — `None`
    /// meaning however long the piece turns out to be.
    ///
    /// `bpm` is the tempo the piece is **rendered** at, which under a
    /// `stretch` fit is not the one written down.
    fn frames(self, bpm: f32) -> (usize, Option<usize>) {
        let seconds = match self.unit {
            Unit::Beats => 60.0 / bpm.max(f32::MIN_POSITIVE),
            Unit::Seconds => 1.0,
        };
        let frame = |value: f32| (value * seconds * RATE).round().max(0.0) as usize;
        (frame(self.span.start), self.span.end.map(frame))
    }
}

/// Two numbers with a colon between them: `12:48` is 12 up to but not
/// including 48, `12:` runs to the end of the piece, `:48` from its start.
///
/// End-exclusive, and spelled the way `render --range` already spells a slice
/// of the timeline: one grammar for "part of it" across the program is one
/// fewer thing to look up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    start: f32,
    end: Option<f32>,
}

impl Span {
    /// A span from `start`, ending at `end` or running to the end of the
    /// piece.
    pub fn new(start: f32, end: Option<f32>) -> Result<Self, SpanError> {
        if !start.is_finite() || start < 0.0 {
            return Err(SpanError::Malformed);
        }
        match end {
            Some(end) if !end.is_finite() || end <= start => Err(SpanError::Backwards),
            _ => Ok(Self { start, end }),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.end {
            Some(end) => write!(f, "{}:{end}", self.start),
            None => write!(f, "{}:", self.start),
        }
    }
}

impl FromStr for Span {
    type Err = SpanError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (start, end) = text.split_once(':').ok_or(SpanError::Malformed)?;
        let part = |text: &str| -> Result<Option<f32>, SpanError> {
            let text = text.trim();
            if text.is_empty() {
                return Ok(None);
            }
            text.parse::<f32>()
                .map(Some)
                .map_err(|_| SpanError::Malformed)
        };
        Self::new(part(start)?.unwrap_or(0.0), part(end)?)
    }
}

/// Text that is not a span.
///
/// Written out by hand rather than derived, because this crate has no error
/// derive to reach for — [`SynthError`] says its own wording in one place too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    /// No colon, a number that is not one, or a negative start.
    Malformed,
    /// Includes the equal case: end-exclusive means `8:8` selects nothing.
    Backwards,
}

impl fmt::Display for SpanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("expected a range like `0:32`, `16:`, or `:32`"),
            Self::Backwards => f.write_str("a range has to end after it begins"),
        }
    }
}

impl std::error::Error for SpanError {}

/// An [`Excerpt`] resolved against the song and the tempo it will be rendered
/// at: which samples are kept, how far the renderer has to go to make them
/// exactly, and which tracks reach the mix.
pub(crate) struct Scope {
    from: usize,
    to: Option<usize>,
    /// The last sample-frame worth putting a note at. Past it nothing rendered
    /// can reach anything kept — see the module doc.
    until: Option<usize>,
    /// One flag per track, in track order: whether it is heard in the mix.
    heard: Vec<bool>,
}

impl Scope {
    /// Resolves `excerpt` against `song` at the tempo it renders at.
    pub(crate) fn of(song: &Song, excerpt: &Excerpt, bpm: f32) -> Result<Self, SynthError> {
        for name in &excerpt.only {
            if !song.tracks.iter().any(|track| &track.name == name) {
                return Err(SynthError::UnknownSoloTrack {
                    track: name.clone(),
                });
            }
        }
        let heard: Vec<bool> = song
            .tracks
            .iter()
            .map(|track| excerpt.only.is_empty() || excerpt.only.contains(&track.name))
            .collect();
        let (from, to) = excerpt
            .window
            .map_or((0, None), |window| window.frames(bpm));
        Ok(Self {
            from,
            to,
            until: to.map(|to| to + guard(song)),
            heard,
        })
    }

    /// Whether this track reaches the mix at all.
    pub(crate) fn heard(&self, track: usize) -> bool {
        self.heard.get(track).copied().unwrap_or(true)
    }

    /// How many tracks are heard — what decides whether a per-track table says
    /// anything the summary above it does not.
    pub(crate) fn heard_count(&self) -> usize {
        self.heard.iter().filter(|heard| **heard).count()
    }

    /// Whether a note starting at this sample-frame can still affect anything
    /// kept.
    pub(crate) fn reaches(&self, at: usize) -> bool {
        self.until.is_none_or(|until| at <= until)
    }

    /// The stretch of a buffer of `frames` this keeps, clamped to it.
    pub(crate) fn keep(&self, frames: usize) -> (usize, usize) {
        let from = self.from.min(frames);
        (from, self.to.unwrap_or(frames).clamp(from, frames))
    }

    /// Where the kept stretch starts, in seconds — what the section rows of a
    /// report are measured from.
    pub(crate) fn opens_at_seconds(&self) -> f64 {
        self.from as f64 / f64::from(RATE)
    }
}

/// How far past its own end a window has to render for the stages that look
/// ahead to see what a whole render would have shown them.
///
/// Three terms, each the reach of one non-causal thing in the path, and they
/// **add** because they run in series: a track's compressors duck ahead of
/// their key, the song chain's duck ahead of the sum, and the limiter ducks
/// ahead of that. `SEAM` joins them because a truncation to a `fit` length is
/// faded over it, and a window ending inside that fade has to be on the same
/// side of the cut as the whole render was.
fn guard(song: &Song) -> usize {
    let track = song
        .tracks
        .iter()
        .map(|track| fx::lookahead_seconds(&track.fx))
        .fold(0.0, f32::max);
    let seconds = track + fx::lookahead_seconds(&song.fx) + fx::limiter::LOOKAHEAD + SEAM;
    (seconds * RATE).ceil().max(0.0) as usize + RECONSTRUCTION
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::song::{PatchRef, Track};

    /// A song of two silent tracks — enough for a scope, which reads names,
    /// chains and a tempo and never a note.
    fn two_tracks() -> Song {
        let track = |name: &str| Track {
            name: name.to_owned(),
            patch: PatchRef::Named("nowhere".to_owned()),
            gain: 1.0,
            pan: 0.0,
            fx: Vec::new(),
        };
        Song {
            bpm: 120.0,
            seed: 0,
            key: None,
            tracks: vec![track("bass"), track("pad")],
            patterns: BTreeMap::new(),
            arrangement: Vec::new(),
            swing: 0.0,
            humanize: None,
            fx: Vec::new(),
            automation: Vec::new(),
            fit: None,
            fade: None,
            tail: None,
        }
    }

    fn scope(excerpt: &Excerpt) -> Scope {
        Scope::of(&two_tracks(), excerpt, 120.0).expect("the excerpt resolves")
    }

    /// The saving, asserted where it can be: past the guard a note cannot
    /// reach anything kept, so it is never synthesised. Without this the
    /// window would still be *correct* and would cost what a whole bake costs,
    /// which is the entire point of it missing.
    #[test]
    fn a_note_past_the_window_and_its_guard_is_not_rendered() {
        let scope = scope(&Excerpt::of(Window::seconds(
            Span::new(0.0, Some(1.0)).unwrap(),
        )));
        let end = RATE as usize;
        assert!(
            scope.reaches(end),
            "a note at the window's edge still plays"
        );
        assert!(
            scope.reaches(end + RECONSTRUCTION),
            "the guard reaches past the edge"
        );
        assert!(
            !scope.reaches(end + RATE as usize),
            "a note a second late is still being rendered"
        );
    }

    /// An open window has no end, so nothing is ever late for it.
    #[test]
    fn an_open_window_never_calls_a_note_late() {
        let scope = scope(&Excerpt::of(Window::beats(Span::new(4.0, None).unwrap())));
        assert!(scope.reaches(usize::MAX));
        let frames = RATE as usize * 5;
        assert_eq!(scope.keep(frames), (RATE as usize * 2, frames));
    }

    /// A solo names the tracks that are heard, and nothing else.
    #[test]
    fn a_solo_hears_the_tracks_it_names_and_no_others() {
        let scope = scope(&Excerpt::only(vec!["pad".to_owned()]));
        assert!(!scope.heard(0), "the bass was not asked for");
        assert!(scope.heard(1), "the pad was");
        assert_eq!(scope.heard_count(), 1);
        assert_eq!(scope.keep(1_000), (0, 1_000), "a solo keeps every sample");
    }

    #[test]
    fn a_whole_render_hears_everything() {
        let scope = scope(&Excerpt::default());
        assert_eq!(scope.heard_count(), 2);
        assert_eq!(scope.opens_at_seconds(), 0.0);
        assert!(Excerpt::default().is_whole());
    }

    #[test]
    fn a_span_reads_the_three_ways_it_is_written() {
        assert_eq!(
            "0:32".parse::<Span>(),
            Ok(Span::new(0.0, Some(32.0)).unwrap())
        );
        assert_eq!("16:".parse::<Span>(), Ok(Span::new(16.0, None).unwrap()));
        assert_eq!(
            ":32".parse::<Span>(),
            Ok(Span::new(0.0, Some(32.0)).unwrap())
        );
    }

    #[test]
    fn a_span_that_selects_nothing_is_refused() {
        assert_eq!("8:8".parse::<Span>(), Err(SpanError::Backwards));
        assert_eq!("8:4".parse::<Span>(), Err(SpanError::Backwards));
        assert_eq!("8".parse::<Span>(), Err(SpanError::Malformed));
        assert_eq!("-1:8".parse::<Span>(), Err(SpanError::Malformed));
    }

    /// Beats are the piece's own unit, so a window in them converts at the
    /// tempo the piece is rendered at and nowhere else.
    #[test]
    fn beats_become_samples_at_the_rendered_tempo() {
        let window = Window::beats(Span::new(4.0, Some(8.0)).unwrap());
        let (from, to) = window.frames(120.0);
        assert_eq!(
            from,
            RATE as usize * 2,
            "four beats at 120 bpm is two seconds"
        );
        assert_eq!(to, Some(RATE as usize * 4));
    }

    #[test]
    fn seconds_are_seconds_whatever_the_tempo() {
        let window = Window::seconds(Span::new(0.0, Some(12.0)).unwrap());
        assert_eq!(window.frames(75.0).1, Some(RATE as usize * 12));
    }

    /// The open end is the whole rest of the piece, so nothing is skipped for
    /// being late and there is no guard to compute.
    #[test]
    fn an_open_window_renders_every_note() {
        let window = Window::beats(Span::new(4.0, None).unwrap());
        assert_eq!(window.frames(120.0).1, None);
    }
}
