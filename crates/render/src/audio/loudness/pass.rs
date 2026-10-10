//! One read of a raw mix on disk: measured, or raised and limited into
//! another file.
//!
//! A run at a time, never whole, for the reason every other pass over the mix
//! gives: an hour of stereo float is over a gigabyte, and none of this needs
//! more than a run of it and the limiter's lookahead.

use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

use scorsese_zimmer::level::Integrated;

use super::limiter::Limiter;
use crate::audio::mix::CHANNELS;

/// How many bytes of the mix are read at a time: a whole number of frames.
const CHUNK: usize = 1 << 16;

/// What raising a mix came to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Raised {
    /// The integrated loudness of what was written, in LUFS.
    pub(crate) lufs: Option<f64>,
    /// How far the limiter's deepest reduction went, in dB.
    pub(crate) limited_db: f64,
}

/// The integrated loudness of the raw mix at `path`, sampled at `rate`.
pub(crate) fn loudness(path: &Path, rate: u32) -> io::Result<Option<f64>> {
    let mut meter = Integrated::new(CHANNELS, rate);
    runs(path, |samples| meter.feed(samples))?;
    Ok(meter.finish())
}

/// Writes the raw mix at `from` to `to`, multiplied by `gain_db` and held
/// under `ceiling_dbtp` by the [`Limiter`], and measures what it wrote.
pub(crate) fn raise(
    from: &Path,
    to: &Path,
    rate: u32,
    gain_db: f64,
    ceiling_dbtp: f64,
) -> io::Result<Raised> {
    let gain = 10_f64.powf(gain_db / 20.0) as f32;
    let mut limiter = Limiter::new(CHANNELS, rate, ceiling_dbtp);
    let mut meter = Integrated::new(CHANNELS, rate);
    let mut writer = BufWriter::new(File::create(to)?);
    let mut raised = Vec::new();
    let mut limited = Vec::new();
    let mut write = |samples: &[f32], writer: &mut BufWriter<File>| -> io::Result<()> {
        meter.feed(samples);
        for sample in samples {
            writer.write_all(&sample.to_le_bytes())?;
        }
        Ok(())
    };
    let mut failed = None;
    runs(from, |samples| {
        if failed.is_some() {
            return;
        }
        raised.clear();
        raised.extend(samples.iter().map(|s| s * gain));
        limited.clear();
        limiter.feed(&raised, &mut limited);
        if let Err(error) = write(&limited, &mut writer) {
            failed = Some(error);
        }
    })?;
    if let Some(error) = failed {
        return Err(error);
    }
    limited.clear();
    limiter.finish(&mut limited);
    write(&limited, &mut writer)?;
    writer.flush()?;
    Ok(Raised {
        lufs: meter.finish(),
        limited_db: limiter.deepest_db(),
    })
}

/// Hands `take` the samples of the raw `f32` file at `path`, a run at a time.
fn runs(path: &Path, mut take: impl FnMut(&[f32])) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut bytes = vec![0_u8; CHUNK];
    let mut samples = Vec::with_capacity(CHUNK / size_of::<f32>());
    loop {
        let filled = fill(&mut file, &mut bytes)?;
        if filled == 0 {
            return Ok(());
        }
        samples.clear();
        samples.extend(
            bytes[..filled]
                .chunks_exact(size_of::<f32>())
                .map(|word| f32::from_le_bytes(word.try_into().expect("four bytes"))),
        );
        take(&samples);
    }
}

/// Reads until `bytes` is full or the file ends, so a run never ends part way
/// through a sample.
fn fill(file: &mut impl Read, bytes: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < bytes.len() {
        match file.read(&mut bytes[filled..])? {
            0 => break,
            read => filled += read,
        }
    }
    Ok(filled)
}
