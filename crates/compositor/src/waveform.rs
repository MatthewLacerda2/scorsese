//! A waveform: amplitude over time, drawn so a client with no ears can read a
//! sound file.
//!
//! The direct analogue of [`crate::sheet`], one medium over. A contact sheet
//! answers *what is in this footage?* for something that cannot watch; this
//! answers *how did this come out?* for something that cannot listen. It is the
//! line anybody who has opened an audio editor already knows how to read: where
//! a voice is speaking, where it stops, where it drops to a whisper.
//!
//! **What the picture is accountable for**: is it silent, is it clipped, does
//! it start late or end early, is it the length it should be, where are the
//! pauses. Those are exactly the questions a duration and a byte hash cannot
//! tell apart — 3.67 seconds of narration and 3.67 seconds of silence agree on
//! every one of them — and every one is a shape on this line.
//!
//! **What it is not accountable for**: the wrong voice, the wrong language, a
//! mispronunciation, a flat read. Those need hearing. Nothing here pretends
//! otherwise, and a picture that claimed to answer them would be worse than no
//! picture at all.
//!
//! Drawing only. Where the numbers come from is `scorsese-render`'s, the way
//! decoding always is.

use scorsese_core::{Anchor, AnchorX, AnchorY, Rgba, TextAlign};

use crate::frame::{BYTES_PER_PIXEL, Frame, Resolution, ResolutionError};
use crate::text::{self, Band, Font, Style};

/// The picture's width. Time is the long axis and a waveform squeezed into a
/// square is a smear.
const WIDTH: u32 = 960;
/// The picture's height. Amplitude has one dimension and gains nothing from
/// more room than the eye needs to see a gap in it.
const HEIGHT: u32 = 260;

/// Where the waveform's own area starts and stops, leaving a caption above it
/// and a time axis below.
const WAVE_TOP: u32 = 36;
/// The row of the bottom rule, and the first row the axis may use.
const WAVE_BOTTOM: u32 = 216;

/// Behind everything. Dark, because a waveform reads as light on dark
/// everywhere anybody has seen one before.
const BACKGROUND: Rgba = Rgba::opaque(0x14, 0x16, 0x1a);
/// The peak envelope: the loudest sample in each column.
const PEAK: Rgba = Rgba::opaque(0x3f, 0x7d, 0xad);
/// The RMS body drawn inside it, which is what the ear calls loudness. The two
/// together read as one shape with a bright core.
const BODY: Rgba = Rgba::opaque(0x9e, 0xd8, 0xff);
/// A column that reached full scale. Unmissable, because clipping is the one
/// thing on this picture that is always a fault.
const CLIPPED: Rgba = Rgba::opaque(0xe0, 0x5a, 0x50);
/// The silence line down the middle, the full-scale rules at the edges, and the
/// axis ticks.
const RULE: Rgba = Rgba::opaque(0x3a, 0x40, 0x4a);
/// Captions and axis labels.
const INK: Rgba = Rgba::opaque(0xc8, 0xcf, 0xd8);

/// Em size of the caption and the axis labels, in pixels.
const LABEL_SIZE: f32 = 15.0;

/// One column of the picture: how loud this slice of the file got, and how
/// loud it was on average.
///
/// Both are absolute amplitudes in `0.0..=1.0`, where `1.0` is full scale. Two
/// numbers rather than one because they answer different questions. The peak
/// says whether anything clipped; the RMS says whether anybody was speaking. A
/// mean alone would hide a click, and a peak alone would make a room tone with
/// one door slam in it look continuously loud.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Column {
    /// The loudest sample in this slice.
    pub peak: f32,
    /// The root-mean-square of the slice.
    pub rms: f32,
}

/// What to draw, and what to write along the edges of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wave<'a> {
    /// One entry per column of the picture, left to right.
    pub columns: &'a [Column],
    /// How long the file is, for the axis labels.
    pub seconds: f64,
    /// One line along the top: what this is a picture of, and how it came out.
    pub caption: &'a str,
}

/// How many columns a waveform picture has, and so how many slices to cut a
/// file into.
///
/// Published because the caller does the cutting, and one column per pixel is
/// what keeps the picture a faithful summary rather than a resampling of a
/// resampling.
pub const COLUMNS: usize = WIDTH as usize;

/// Draws the waveform.
///
/// Fewer columns than [`COLUMNS`] are drawn as far as they go, leaving the rest
/// of the width empty. That is honest rather than stretched: a file cut into
/// fewer slices than there are pixels has not got a waveform across the width.
pub fn draw(wave: &Wave<'_>, font: &Font) -> Result<Frame, ResolutionError> {
    let mut frame = Frame::black(Resolution::new(WIDTH, HEIGHT)?);
    frame.fill(BACKGROUND);

    // Rules before bars, so a silent stretch shows the middle line through it
    // rather than covering it — "flat at zero" and "nothing drawn here" have to
    // look different.
    for row in [WAVE_TOP, middle(), WAVE_BOTTOM] {
        frame.fill_rows(row..row + 1, RULE);
    }
    for x in [0, WIDTH / 2, WIDTH - 1] {
        column(&mut frame, x, WAVE_BOTTOM..WAVE_BOTTOM + 6, RULE);
    }
    for (x, one) in wave.columns.iter().take(COLUMNS).enumerate() {
        bars(&mut frame, x as u32, *one);
    }

    label(&mut frame, wave.caption, font, 6.0, TextAlign::Left);
    for (align, at) in [
        (TextAlign::Left, 0.0),
        (TextAlign::Center, wave.seconds / 2.0),
        (TextAlign::Right, wave.seconds),
    ] {
        label(&mut frame, &stamp(at), font, 226.0, align);
    }
    Ok(frame)
}

/// The row a silent sample sits on.
const fn middle() -> u32 {
    (WAVE_TOP + WAVE_BOTTOM) / 2
}

/// One column's peak envelope, with its RMS body drawn inside it.
fn bars(frame: &mut Frame, x: u32, one: Column) {
    let peak = if one.peak >= 1.0 { CLIPPED } else { PEAK };
    bar(frame, x, one.peak, peak);
    bar(frame, x, one.rms, BODY);
}

/// A vertical run of pixels reaching `amplitude` of full scale either side of
/// the silence line.
///
/// At least one pixel tall whatever the amplitude, so near-silence draws as a
/// thin line rather than as nothing at all — "quiet" and "no samples here" are
/// different findings and the picture has to keep them apart.
fn bar(frame: &mut Frame, x: u32, amplitude: f32, colour: Rgba) {
    let half = f64::from(WAVE_BOTTOM - WAVE_TOP) / 2.0;
    let reach = half * f64::from(amplitude.clamp(0.0, 1.0));
    let top = (f64::from(middle()) - reach).round().max(0.0) as u32;
    let bottom = ((f64::from(middle()) + reach).round().max(0.0) as u32).max(top + 1);
    column(frame, x, top..bottom.min(HEIGHT), colour);
}

/// Paints one pixel column over a range of rows.
fn column(frame: &mut Frame, x: u32, rows: std::ops::Range<u32>, colour: Rgba) {
    if x >= WIDTH {
        return;
    }
    let channels = colour.channels();
    let bytes = frame.bytes_mut();
    for y in rows.start..rows.end.min(HEIGHT) {
        let at = (y as usize * WIDTH as usize + x as usize) * BYTES_PER_PIXEL;
        bytes[at..at + BYTES_PER_PIXEL].copy_from_slice(&channels);
    }
}

/// A moment as `s.ts` under a minute and `m:ss.t` over it — the resolution
/// somebody checking whether a line starts late actually needs.
fn stamp(at: f64) -> String {
    if at < 60.0 {
        return format!("{at:.1}s");
    }
    let minutes = (at / 60.0).floor();
    format!("{minutes:.0}:{:04.1}", at - minutes * 60.0)
}

/// One line of text at `top`, set in the picture's ink.
///
/// The alignment is what places it: three stamps against the same width, at the
/// left, the centre and the right, land under the three ticks. Ruling a tick
/// per second would be a grey band across sixteen minutes and one tick across
/// four seconds; three read the same at either length.
fn label(frame: &mut Frame, words: &str, font: &Font, top: f32, align: TextAlign) {
    const MARGIN: f32 = 10.0;
    let style = Style {
        size: LABEL_SIZE,
        color: INK,
        align,
        line_height: LABEL_SIZE * 1.2,
        max_width: WIDTH as f32 - MARGIN * 2.0,
        // Drawn on a plain background this picture chose itself.
        edge: None,
        anchor: Anchor {
            x: AnchorX::Left,
            y: AnchorY::Top,
        },
    };
    text::draw_in(
        frame,
        words,
        font,
        &style,
        Band {
            top,
            height: LABEL_SIZE * 1.6,
        },
    );
}
