//! Summing sources into one buffer of samples.
//!
//! Pure arithmetic over slices: no ffmpeg, no files, no timeline. That is
//! deliberate — mixing is the audio half of Path B, the counterpart to
//! `scorsese-compositor` producing a picture frame from source frames, and
//! keeping it free of I/O is what lets it be tested by asking about numbers.

use std::io::{self, Write};

use super::gain::Gain;
use scorsese_soundgen::level::Meter;

/// How many channels the mix is produced in.
///
/// Stereo, and not yet a setting: mono and surround are real asks, but each
/// needs a downmix/upmix policy per source, and inventing one before anything
/// needs it would be a knob nobody sets.
pub const CHANNELS: usize = 2;

/// Where a run of samples sits inside its clip, so gain can be evaluated for
/// each one.
#[derive(Debug, Clone, Copy)]
pub struct Ramp {
    /// Clip-relative time of the first sample, in timeline frames.
    pub first: f64,
    /// How far the clip-relative time advances per sample, in timeline frames.
    pub per_sample: f64,
}

/// An accumulating buffer of interleaved samples.
///
/// Held as `f32` rather than the integers that go into the file: summing two
/// sources that each peak near full scale overshoots, and floats carry the
/// overshoot so it can be dealt with once, at the end, rather than wrapping
/// silently at every addition.
#[derive(Debug, Clone, PartialEq)]
pub struct Mix {
    samples: Vec<f32>,
}

impl Mix {
    /// A silent buffer `frames` sample-frames long.
    pub fn silence(frames: usize) -> Self {
        Self {
            samples: vec![0.0; frames * CHANNELS],
        }
    }

    /// How many sample-frames this holds.
    pub fn frames(&self) -> usize {
        self.samples.len() / CHANNELS
    }

    /// Adds one source's samples, scaled by its volume over the run, telling
    /// `meter` what was added.
    ///
    /// A source shorter than the buffer contributes only what it has: the
    /// remainder is left as it was, which is silence unless another source
    /// covers it. That is what running out of audio means, and it is the audio
    /// counterpart of a layer whose footage ran short.
    ///
    /// The meter is fed here, and not by the caller from `source`, because what
    /// is worth knowing is **what this clip put into the mix** — the samples
    /// after its own volume keyframes. Measuring `source` instead would report
    /// the file rather than the edit, and a clip faded to nothing would read as
    /// loud as its media.
    pub fn add(&mut self, source: &[f32], gain: Gain<'_>, ramp: Ramp, meter: &mut Meter) {
        let shared = self.samples.len().min(source.len());
        if gain.is_unity() {
            meter.feed(&source[..shared]);
            for (into, &sample) in self.samples[..shared].iter_mut().zip(source) {
                *into += sample;
            }
            return;
        }
        let mut scaled = Vec::with_capacity(shared);
        for (frame, (into, from)) in self.samples[..shared]
            .chunks_mut(CHANNELS)
            .zip(source[..shared].chunks(CHANNELS))
            .enumerate()
        {
            // Once per sample-frame, not once per channel: both channels of one
            // instant are at the same point in the fade.
            let level = gain.at(ramp.first + ramp.per_sample * frame as f64);
            for (into, &sample) in into.iter_mut().zip(from) {
                *into += sample * level;
                scaled.push(sample * level);
            }
        }
        meter.feed(&scaled);
    }

    /// Writes the mix out as little-endian `f32`, which is what the encoder is
    /// told to expect.
    ///
    /// **Sums past full scale are hard-clipped**, deliberately and only here.
    /// The alternatives are worse: normalising the whole render would make the
    /// output level depend on its loudest instant, and a limiter would change
    /// the dynamics of a mix the author balanced. Clipping is audible, which is
    /// the point — it is a mix that needs turning down, and quietly rescaling it
    /// would hide that.
    pub fn write_to(&self, out: &mut impl Write) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(self.samples.len() * size_of::<f32>());
        for &sample in &self.samples {
            bytes.extend_from_slice(&sample.clamp(-1.0, 1.0).to_le_bytes());
        }
        out.write_all(&bytes)
    }

    /// The samples as they stand, for tests and for anything that wants to look
    /// before they are written.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}
