//! Keeping a lossy delivery under full scale.
//!
//! A lossy encoder does not hand back the samples it was given. It keeps what
//! it can afford of the spectrum and rebuilds a waveform from that, and the
//! rebuilt peaks land **above** the originals — by about 1 dB on one synth
//! score and 3.3 dB on a denser one, measured on two real projects (#503). A
//! mix that sat at −1.2 dBTP came out of AAC at +1.9, over full scale, in the
//! file a viewer plays. Nothing in the mix was wrong; the codec added it.
//!
//! **Where the headroom lives: in the render, measured rather than assumed.**
//! Because the overshoot depends on the material, no fixed margin is right —
//! one wide enough for the dense score turns every quiet one down for nothing,
//! and one that fits the quiet ones clips the dense score. So before the real
//! encode, the finished mix is *rehearsed*: encoded on its own with the codec,
//! bitrate and container the delivery will use, decoded, and metered. When
//! that comes back over [`DELIVERY_CEILING_DBTP`], the whole mix is turned down
//! by exactly the excess and rehearsed again. What the encoder then delivers is
//! what was rehearsed.
//!
//! **A uniform trim, never a limiter**, for the reason [`super::Mix`] gives for
//! not limiting the mix itself: a limiter changes the dynamics of a mix the
//! author balanced, and a trim changes only where the whole of it sits. The
//! trim is the *codec's* cost, so it is paid only when the codec is lossy and
//! only as far as this material needs — a lossless delivery, and a lossy one
//! that already fits, are left exactly as mixed. It is reported
//! ([`Trim`]) rather than done quietly, and the mix's own level in the report
//! stays the level the author mixed, so a mix that is too hot is still said to
//! be.
//!
//! The mix is the one place to do it: it is the final sum, after every clip,
//! volume keyframe and duck has had its say, and the assets it came from — a
//! bake held at −1 dBTP by its own limiter — are lossless and correct as they
//! are.

use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use scorsese_zimmer::level::{Loudness, Meter};

use super::mix::CHANNELS;
use super::read;
use crate::error::RenderError;
use crate::pipe::encode_mix;
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// The loudest a lossy delivery's soundtrack may come back out of its decoder,
/// in dB true peak.
///
/// The same −1 dBTP a synthesis bake is limited to, so the promise a bake makes
/// about its own file is the promise the delivered file keeps. Judged on the
/// **decoded** output, since that is the waveform a viewer's player reproduces.
pub const DELIVERY_CEILING_DBTP: f64 = -1.0;

/// How many times a mix is rehearsed before the render gives up improving it.
///
/// A codec's overshoot is close to proportional to the level it is fed, so one
/// trim by the measured excess nearly always lands under the ceiling and the
/// second rehearsal only confirms it. The third is for the material where it
/// was not quite proportional. Past that the delivery is measured and reported
/// as it came out, which is a finding rather than a failure.
const REHEARSALS: usize = 3;

/// How many bytes of the mix are rescaled at a time.
const CHUNK: usize = 1 << 16;

/// What a render did to fit its soundtrack under a lossy codec's overshoot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trim {
    /// The codec that needed the room.
    pub codec: &'static str,
    /// Where the untrimmed mix came back out of that codec, in dB true peak.
    pub encoded_dbtp: f64,
    /// How far the whole mix was turned down, in dB. Always positive.
    pub trimmed_db: f64,
}

impl std::fmt::Display for Trim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "turned down {:.1} dB: {} would have peaked at {:+.1} dBTP, over the \
             {DELIVERY_CEILING_DBTP:.0} dBTP ceiling",
            self.trimmed_db, self.codec, self.encoded_dbtp
        )
    }
}

/// How far a delivery peaking at `true_peak_dbfs` has to come down, in dB, or
/// `None` when it already fits.
pub(crate) fn excess(true_peak_dbfs: Option<f64>) -> Option<f64> {
    true_peak_dbfs
        .map(|peak| peak - DELIVERY_CEILING_DBTP)
        .filter(|over| *over > 0.0)
}

/// Rehearses the mix at `mix` through the delivery's codec and turns it down
/// until what comes back fits under [`DELIVERY_CEILING_DBTP`].
///
/// `None` when nothing was changed — the codec is lossless, the mix is silent,
/// or it already fits. `out` names the delivery; the rehearsal is written
/// beside it and removed before this returns.
pub(crate) fn fit(
    tools: &Tools,
    settings: &RenderSettings,
    mix: &Path,
    out: &Path,
) -> Result<Option<Trim>, RenderError> {
    let codec = settings.format.audio();
    if !codec.is_lossy() {
        return Ok(None);
    }
    let rehearsal = Rehearsal(super::scratch_beside(out, "scorsese-rehearsal"));
    let mut trim: Option<Trim> = None;
    for _ in 0..REHEARSALS {
        encode_mix(tools, settings, mix, &rehearsal.0)?;
        let peak = measure(tools, &rehearsal.0)?.true_peak_dbfs;
        let Some(over) = excess(peak) else { break };
        scale(mix, gain_of(over)).map_err(|source| RenderError::Scratch {
            path: mix.to_owned(),
            source,
        })?;
        let trim = trim.get_or_insert(Trim {
            codec: codec.name(),
            encoded_dbtp: peak.unwrap_or_default(),
            trimmed_db: 0.0,
        });
        trim.trimmed_db += over;
    }
    Ok(trim)
}

/// How loud the soundtrack of a finished file is, decoded as a viewer's player
/// would decode it.
///
/// Used on the delivery itself once it is written, which is what makes the
/// render's report about the file rather than about the mix it was made from.
pub(crate) fn measure(tools: &Tools, file: &Path) -> Result<Loudness, RenderError> {
    let mut meter = Meter::new(CHANNELS);
    read::decode(tools, file, CHANNELS, |samples| meter.feed(samples))?;
    Ok(meter.finish())
}

/// The linear factor that lowers a signal by `db` decibels.
fn gain_of(db: f64) -> f32 {
    10_f64.powf(-db / 20.0) as f32
}

/// Multiplies every sample of the raw `f32` mix at `path` by `gain`, in place.
///
/// A run at a time rather than read whole: an hour of stereo float is well over
/// a gigabyte, and a trim needs to hold none of it.
fn scale(path: &Path, gain: f32) -> io::Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut bytes = vec![0_u8; CHUNK];
    let mut at = 0_u64;
    loop {
        file.seek(SeekFrom::Start(at))?;
        let filled = fill(&mut file, &mut bytes)?;
        if filled == 0 {
            return Ok(());
        }
        for word in bytes[..filled].chunks_exact_mut(size_of::<f32>()) {
            let sample = f32::from_le_bytes(word.try_into().expect("four bytes")) * gain;
            word.copy_from_slice(&sample.to_le_bytes());
        }
        file.seek(SeekFrom::Start(at))?;
        file.write_all(&bytes[..filled])?;
        at += filled as u64;
    }
}

/// Reads until `bytes` is full or the file ends, so a chunk never ends part way
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

/// The rehearsal file, removed when dropped — including when a render fails
/// part way through a rehearsal.
struct Rehearsal(PathBuf);

impl Drop for Rehearsal {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_peak_under_the_ceiling_needs_nothing() {
        assert_eq!(excess(Some(-1.5)), None);
        assert_eq!(excess(Some(DELIVERY_CEILING_DBTP)), None);
        assert_eq!(excess(None), None, "silence fits anywhere");
    }

    #[test]
    fn a_peak_over_the_ceiling_comes_down_by_exactly_the_excess() {
        let over = excess(Some(2.0)).expect("over");
        assert!((over - 3.0).abs() < 1e-9);
        assert!((f64::from(gain_of(6.02)) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn scaling_rewrites_every_sample_across_chunk_boundaries() {
        let dir = std::env::temp_dir().join(format!("scorsese-503-scale-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("mix.pcm");
        let samples: Vec<f32> = (0..CHUNK).map(|i| i as f32 / CHUNK as f32).collect();
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        std::fs::write(&path, bytes).expect("written");

        scale(&path, 0.5).expect("scaled");

        let back = std::fs::read(&path).expect("read");
        std::fs::remove_dir_all(&dir).ok();
        for (word, &was) in back.chunks_exact(4).zip(&samples) {
            let now = f32::from_le_bytes(word.try_into().expect("four bytes"));
            assert_eq!(now, was * 0.5);
        }
    }
}
