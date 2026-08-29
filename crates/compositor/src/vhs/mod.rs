//! The tape: five artefacts that only ever arrive together.
//!
//! Colour smeared sideways because chroma was recorded at a fraction of luma's
//! bandwidth ([`chroma`]); snow on the luma and on the colour differences;
//! scanlines and the torn head-switching band ([`lines`]); and a tracking
//! wobble ([`tracking`]). Each is a number on the clip, all five run `0.0` to
//! `1.0`, and [`scorsese_core::Vhs::mono`] decides whether the chroma path is
//! modelled at all — which is a mode with two honest positions and not a
//! preset. That type's own doc carries why this is one named effect with five
//! sub-values rather than one knob or five loose properties.
//!
//! **Computed, never composited.** There is no overlay footage here, no noise
//! plate, no imported asset of any kind: every artefact is arithmetic over the
//! layer's own pixels, which is what makes the look survive `scp -r` and cost
//! nothing to render.
//!
//! **Nothing here reads a frame that is not this one.** The wobble and the tear
//! vary over time as well as down the picture, and both do it as pure functions
//! of the seed [`crate::Properties`] resolved from the clip and the frame — the
//! same mechanism grain uses. Nothing is carried between frames, so a frame can
//! be drawn by any worker in any order and two renders of one project are the
//! same picture.
//!
//! **Softness and ringing are not here**, and that is a decision rather than an
//! omission. A tape is soft, and [`crate::blur`] already softens a layer
//! honestly and composes with this — one field on the same clip. What would be
//! left is the *ringing*: the overshoot a sharpener leaves on either side of an
//! edge, which is a second filter kernel for a halo most people would read as
//! "somebody sharpened this". Five knobs that each name an artefact anybody can
//! point at is worth more than six where the sixth needs explaining.
//!
//! **Premultiplied pixels in, premultiplied pixels out**, for the reason
//! [`crate::blur`] and [`crate::aberration`] take them that way: this smears
//! colour along a row and displaces whole rows sideways, so it reads pixels it
//! is not writing, and only in premultiplied form does a transparent one
//! contribute nothing rather than whatever colour it was stored as. Everything
//! this *adds* — the snow — is scaled by the pixel's own alpha, which is what
//! adding a colour to a premultiplied pixel means.

pub(crate) mod chroma;
pub(crate) mod lines;
pub(crate) mod tracking;

use scorsese_core::Vhs;

use crate::frame::{BYTES_PER_PIXEL, Resolution};
use crate::grain::{self, Grain};

use chroma::Sample;
use lines::{Band, Lines};
use tracking::Tracking;

/// Which noise field is which. The luma's and the two colour differences' —
/// three fields of one seed, because one field read three times is one texture
/// at three times the height rather than three textures.
const SNOW: u64 = 0;
const BLUE: u64 = 1;
const RED: u64 = 2;

/// How strong the colour speckle is beside the luma's.
///
/// Below it, because chroma noise reaching the same amplitude as luma noise
/// would be a picture of confetti: the colour differences start at zero on a
/// grey pixel, so the same swing is a far larger *proportional* error there
/// than it is on the brightness.
const SPECKLE: f64 = 0.5;

/// The scratch a tape needs, kept by the compositor between frames — at 1080p
/// the output alone is eight megabytes, and allocating that thirty times a
/// second is pure churn.
#[derive(Debug, Default)]
pub(crate) struct Buffers {
    /// The taped layer.
    out: Vec<u8>,
    /// One row of the source, on the colour-difference axes.
    row: Vec<Sample>,
    /// That row's colour differences after the smear. A second buffer and not
    /// the first one rewritten, because a trailing average reads a value the
    /// window has not left yet.
    smeared: Vec<(f64, f64)>,
}

/// One layer's tape, resolved for one frame.
///
/// Built once per layer and asked per row and per pixel, so everything that
/// does not vary across the raster is worked out here rather than in the inner
/// loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tape {
    /// The snow on the luma, and on each colour difference. All three are
    /// `None` at once when there is no snow, and the colour pair is `None` on
    /// its own in mono.
    snow: Option<Grain>,
    speckle: Option<(Grain, Grain)>,
    /// How many pixels of a row one chroma sample is smeared across. One is no
    /// smear.
    smear: usize,
    /// Whether the picture has a chroma path at all.
    mono: bool,
    lines: Lines,
    band: Band,
    tracking: Tracking,
}

impl Tape {
    /// The tape a clip asks for, or `None` when it asks for none — so a layer
    /// with the field left alone costs this stage nothing at all.
    ///
    /// `seed` is where this layer's noise fields start at this instant,
    /// resolved by [`crate::Properties`] because that is where the clip and the
    /// frame are both known. It is also what makes the wobble move, so it is
    /// read whether or not there is snow to draw.
    pub(crate) fn new(vhs: Vhs, seed: u64, resolution: Resolution) -> Option<Self> {
        if vhs.is_none() {
            return None;
        }
        let (width, height) = (resolution.width(), resolution.height());
        let snow = Grain::new(grain::field(seed, SNOW), vhs.noise, resolution);
        let speckle = |which, amount| Grain::new(grain::field(seed, which), amount, resolution);
        let speckle = if vhs.mono {
            None
        } else {
            speckle(BLUE, vhs.noise * SPECKLE).zip(speckle(RED, vhs.noise * SPECKLE))
        };
        Some(Self {
            snow,
            speckle,
            smear: chroma::window(vhs.chroma_bleed, width as usize),
            mono: vhs.mono,
            lines: Lines::new(vhs.scanlines, height),
            band: Band::new(vhs.head_switch, height),
            tracking: Tracking::new(seed, vhs.jitter, vhs.head_switch, width, height),
        })
    }
}

/// Runs `source` through the tape into the buffers and hands back the result.
///
/// Returns `source` untouched when there is nothing to do, so the caller can
/// use the answer unconditionally — the same contract [`crate::blur::into`] and
/// [`crate::aberration::into`] have, and for the same two cases: no tape, and a
/// buffer whose length disagrees with the resolution it claims, which is
/// refused a few lines later where every other malformed layer is.
pub(crate) fn into<'a>(
    buffers: &'a mut Buffers,
    source: &'a [u8],
    resolution: Resolution,
    tape: Option<Tape>,
) -> &'a [u8] {
    let (width, height) = (resolution.width() as usize, resolution.height() as usize);
    let Some(tape) = tape else {
        return source;
    };
    if source.len() != width * height * BYTES_PER_PIXEL {
        return source;
    }
    let Buffers { out, row, smeared } = buffers;
    out.clear();
    out.reserve(source.len());
    row.clear();
    row.resize(width, Sample::default());
    smeared.clear();
    smeared.resize(width, (0.0, 0.0));
    for y in 0..height {
        let line = &source[y * width * BYTES_PER_PIXEL..(y + 1) * width * BYTES_PER_PIXEL];
        let switching = tape.band.depth(y);
        // The head-switching band loses its colour along with its position:
        // what is left of the signal there is not a picture with a cast, it is
        // not a picture. So the band is mono whether or not the rest is.
        let grey = tape.mono || switching.is_some();
        // The colour speckle goes on **before** the smear, because it went down
        // the same narrow chroma path the picture did and comes back out
        // streaked the same way — which is what makes tape noise read as
        // coloured streaks rather than as coloured dots.
        for (x, pixel) in line.chunks_exact(BYTES_PER_PIXEL).enumerate() {
            let mut sample = chroma::split(pixel);
            if let Some((blue, red)) = tape.speckle {
                let (index, brightness) = (y * width + x, sample.brightness());
                sample.cb += blue.at(index, brightness) * sample.alpha;
                sample.cr += red.at(index, brightness) * sample.alpha;
            }
            row[x] = sample;
        }
        chroma::bleed(row, smeared, tape.smear);
        let shift = tape.tracking.shift(y, switching);
        let fall = tape.lines.fall(y);
        let last = width as isize - 1;
        for x in 0..width {
            // Off the raster reads the nearest column on it, like the blur's
            // window and the aberration's sample: a displaced row that read
            // black past its edge would draw a bar rather than a tear.
            let from = (x as isize - shift).clamp(0, last) as usize;
            let sample = row[from];
            let (cb, cr) = if grey { (0.0, 0.0) } else { smeared[from] };
            // A scanline darkens the picture, and darkening a picture scales
            // every channel — so it scales the brightness and both colour
            // differences alike, rather than draining the colour out of every
            // other row.
            let scanned = Sample {
                luma: sample.luma * fall,
                cb: cb * fall,
                cr: cr * fall,
                alpha: sample.alpha,
            };
            // The snow goes on last, and after the displacement rather than
            // before it: it is what playback added, so it belongs to the tape
            // and not to the picture the tape is carrying. Weighted by the
            // brightness of the pixel somebody is actually looking at, which is
            // the one after the scanline rather than the one before it.
            let luma = match tape.snow {
                Some(snow) => {
                    scanned.luma + snow.at(y * width + x, scanned.brightness()) * scanned.alpha
                }
                None => scanned.luma,
            };
            chroma::join(out, luma, scanned.cb, scanned.cr, scanned.alpha);
        }
    }
    out.as_slice()
}
