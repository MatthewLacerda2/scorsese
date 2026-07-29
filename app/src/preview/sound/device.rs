//! The output device, and the clock it keeps.
//!
//! **Audio is the clock and the picture follows it.** That inversion is the
//! content of this module. Playback already ran on the wall clock and dropped
//! frames rather than slowing down, because what a preview says about pacing
//! has to stay true when compositing cannot keep up. Sound cannot be treated
//! that way: a dropped sample is a click and a stretched one is a pitch
//! change. So the card's own progress through the mix becomes the authority on
//! where the playhead is, and the picture is whichever frame we manage to draw
//! for it.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use scorsese_render::audio::CHANNELS;

/// A stream feeding the sound card, and how far through the mix it has got.
///
/// Dropping it closes the stream. That is what makes stopping, seeking and
/// closing a project structurally incapable of leaving a voice hanging: there
/// is exactly one owner, and its `Drop` is the stop button.
pub(in crate::preview) struct Voice {
    /// Held for its lifetime alone — dropping it stops the sound.
    _stream: cpal::Stream,
    /// Sample-frames handed to the card so far. Written in the audio callback
    /// and read by the window, so it is atomic rather than locked: a lock in a
    /// callback that must never block is how audio comes out broken.
    played: Arc<AtomicU64>,
}

impl Voice {
    /// How many sample-frames have reached the card.
    pub(super) fn played(&self) -> u64 {
        self.played.load(Ordering::Relaxed)
    }
}

/// The rate the default output device runs at.
///
/// Asked *before* mixing so the mix can be produced in exactly that rate and
/// never resampled again. The mixer already resamples every source on the way
/// in — that is what `RenderSettings::sample_rate` is — so asking it for the
/// card's rate costs nothing and removes any need for a resampler here. A
/// resampler here would be a second piece of audio arithmetic in a project
/// whose rule is that there is never a second one.
///
/// # Errors
///
/// If there is no output device, or it will not say what it wants. Both are
/// ordinary — a machine with no sound card, or a headless CI runner — and the
/// caller plays the picture silently rather than treating it as a fault.
pub(super) fn rate() -> Result<u32, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no audio output device".to_owned())?;
    let config = device
        .default_output_config()
        .map_err(|error| error.to_string())?;
    Ok(config.sample_rate())
}

/// Opens a stream playing `samples`, interleaved at `rate`, entering the mix
/// `skip` sample-frames in.
///
/// `skip` is what keeps the picture from jumping backwards when sound takes
/// over the clock. Making the mix takes a moment, and the picture runs on the
/// wall clock meanwhile; entering the buffer where that clock has got to means
/// the handover moves the playhead by nothing at all. It is also why `played`
/// counts from the start of the mix rather than from the first sample this
/// stream emits — the number is a position, not an amount.
///
/// # Errors
///
/// If no device will take a stereo stream at this rate.
pub(super) fn open(samples: Arc<Vec<f32>>, rate: u32, skip: u64) -> Result<Voice, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no audio output device".to_owned())?;
    let config = cpal::StreamConfig {
        channels: cpal::ChannelCount::try_from(CHANNELS).unwrap_or(2),
        sample_rate: rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let mut cursor = usize::try_from(skip)
        .unwrap_or(usize::MAX)
        .saturating_mul(CHANNELS)
        .min(samples.len());
    let played = Arc::new(AtomicU64::new((cursor / CHANNELS) as u64));
    let counter = Arc::clone(&played);
    let stream = device
        .build_output_stream(
            config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let take = out.len().min(samples.len().saturating_sub(cursor));
                out[..take].copy_from_slice(&samples[cursor..cursor + take]);
                // Past the end of the mix the card still asks for samples, and
                // silence is the honest answer. Leaving the buffer as it came
                // would replay whatever was in it, which is a buzz.
                out[take..].fill(0.0);
                cursor += take;
                counter.store((cursor / CHANNELS) as u64, Ordering::Relaxed);
            },
            // Nothing useful can be done here from the audio thread, and the
            // window finding out a moment later changes nothing it would do:
            // the mix keeps playing or it does not, and the clock says which.
            |_error| {},
            None,
        )
        .map_err(|error| error.to_string())?;
    stream.play().map_err(|error| error.to_string())?;

    Ok(Voice {
        _stream: stream,
        played,
    })
}
