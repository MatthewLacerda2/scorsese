//! Sound for the preview: the mix, the device, and which of them is the clock.
//!
//! **The same mix a render produces.** Every sample here comes from
//! `scorsese_render::audio::mixdown`, the renderer's own mixer, for the same
//! reason the picture comes from its own still: a preview that computes sound
//! its own way is a preview that lies, and the whole value of watching a cut
//! in the window is that what you hear is what ships.
//!
//! **Mixed from the playhead, not from the top.** Pressing play at 0:40 builds
//! a plan starting at 0:40, so nothing before it is decoded. The renderer
//! already takes a `FrameRange`; this asks for one.
//!
//! **Mixed off the window's thread.** Mixing spawns an ffmpeg per audible clip
//! per stretch, which is far too slow to do between two repaints. It happens
//! on a worker; until it lands, the picture plays silently on the wall clock
//! and then the sound takes over the clock when it starts. A window that froze
//! on the play button would be a worse answer than a beat of silence.
//!
//! ## Scrubbing and stepping are silent, deliberately
//!
//! A frame step has no meaningful sound — a thirtieth of a second of audio is
//! a click, not a note. Scrubbing could in principle play, and every editor
//! that does it needs a whole scheme for it: variable-rate playback, grain
//! stitching, or a filter to keep it from screaming. That is a different piece
//! of work with its own design, and inheriting whatever fell out of this one
//! would have been the wrong way to arrive at it. So sound belongs to the
//! transport running, and to nothing else.

mod device;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use scorsese_core::{Fps, Frames, Project};
use scorsese_render::audio::mixdown;
use scorsese_render::plan::{FrameRange, Plan};
use scorsese_render::{RenderSettings, Resolution, SampleRate, Tools};

use device::Voice;

/// The preview's sound, from pressing play to the last sample.
pub(super) enum Sound {
    /// The mix is being made. The picture plays on the wall clock meanwhile.
    Mixing {
        /// Where playback started, so the clock can be measured from it.
        from: Frames,
        /// When it started, so the handover to sound can enter the mix where
        /// the wall clock has got to rather than at the beginning.
        since: std::time::Instant,
        /// The worker's answer, when it has one.
        mixed: Receiver<Result<Vec<f32>, String>>,
        /// The rate the mix is being made at, which is the card's own.
        rate: u32,
    },
    /// The mix is playing, and is the clock.
    Playing {
        /// Where playback started.
        from: Frames,
        /// The card, and its progress through the mix.
        voice: Voice,
        /// The rate it is playing at.
        rate: u32,
    },
    /// There is no sound to play, or none can be played, and the picture runs
    /// on the wall clock. Carries what to say if anyone asks.
    Silent(Option<String>),
}

impl Sound {
    /// Starts making the mix for a play that begins at `from`.
    ///
    /// Never fails: a machine with no sound card, or a project with no audio
    /// in it, plays the picture silently. Sound is what this adds to a
    /// preview, not what a preview now requires.
    pub(super) fn start(project: &Project, root: &Path, from: Frames) -> Self {
        let rate = match device::rate() {
            Ok(rate) => rate,
            Err(why) => return Self::Silent(Some(why)),
        };
        let (send, mixed) = channel();
        // The project is cloned rather than borrowed because the worker
        // outlives this call and the document is edited while it runs. A
        // document is kilobytes of JSON; the mix it produces is megabytes.
        let (project, root) = (project.clone(), root.to_path_buf());
        std::thread::spawn(move || {
            let _ = send.send(mix(&project, &root, from, rate));
        });
        Self::Mixing {
            from,
            since: std::time::Instant::now(),
            mixed,
            rate,
        }
    }

    /// Where the playhead should be, or `None` when sound is not the clock.
    ///
    /// `None` is not a failure — it is the answer while the mix is still being
    /// made, and for a silent project. The caller falls back to the wall clock,
    /// which is what playback did before there was any sound at all.
    pub(super) fn at(&self, fps: Fps) -> Option<Frames> {
        match self {
            Self::Playing { from, voice, rate } => {
                let seconds = voice.played() as f64 / f64::from(*rate);
                Some(*from + fps.frames(seconds))
            }
            Self::Mixing { .. } | Self::Silent(_) => None,
        }
    }

    /// Picks up the mix when the worker has it, and starts playing.
    ///
    /// Called once a repaint. Doing this here rather than on the worker keeps
    /// the stream owned by the window, which is what makes dropping `Sound`
    /// the only stop button there needs to be.
    pub(super) fn poll(&mut self) {
        let Self::Mixing {
            from,
            since,
            mixed,
            rate,
        } = self
        else {
            return;
        };
        let (from, rate) = (*from, *rate);
        // Where the wall clock got to while the mix was being made. Entering
        // the buffer there is what makes the handover invisible.
        let reached = (since.elapsed().as_secs_f64() * f64::from(rate)) as u64;
        *self = match mixed.try_recv() {
            Ok(Ok(samples)) if samples.is_empty() => Self::Silent(None),
            Ok(Ok(samples)) => match device::open(Arc::new(samples), rate, reached) {
                Ok(voice) => Self::Playing { from, voice, rate },
                Err(why) => Self::Silent(Some(why)),
            },
            Ok(Err(why)) => Self::Silent(Some(why)),
            // The worker died without answering, which cannot happen unless it
            // panicked — and a panicked mixer is a silent preview, not a
            // panicked window.
            Err(TryRecvError::Disconnected) => Self::Silent(None),
            Err(TryRecvError::Empty) => return,
        };
    }

    /// Why there is no sound, when there is a reason worth saying.
    pub(super) fn trouble(&self) -> Option<&str> {
        match self {
            Self::Silent(why) => why.as_deref(),
            Self::Mixing { .. } | Self::Playing { .. } => None,
        }
    }
}

/// Mixes from `from` to the end of the edit, at `rate`.
///
/// Returns an empty buffer for a project with nothing audible in it — the
/// difference between a silent film and a film with a silent soundtrack, the
/// same distinction `mixdown` itself makes.
fn mix(project: &Project, root: &Path, from: Frames, rate: u32) -> Result<Vec<f32>, String> {
    let tools = Tools::discover().map_err(|error| error.to_string())?;
    let fps = project.timeline_fps;
    let range = FrameRange::new(from, None).map_err(|error| error.to_string())?;
    let plan = Plan::build(project, fps, range).map_err(|error| error.to_string())?;
    let rate = SampleRate::new(rate).map_err(|error| error.to_string())?;
    // The raster is required to build settings and is never used: nothing here
    // composites a frame. The smallest legal one says so.
    let raster = Resolution::new(2, 2).map_err(|error| error.to_string())?;
    let settings = RenderSettings::new(raster, fps).with_audio(rate, None);

    let scratch = scratch_path();
    let Some((mixdown, _notes)) =
        mixdown(&tools, &settings, &plan, root, &scratch).map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let bytes = std::fs::read(mixdown.path()).map_err(|error| error.to_string())?;
    Ok(bytes
        .chunks_exact(4)
        .map(|four| f32::from_le_bytes([four[0], four[1], four[2], four[3]]))
        .collect())
}

/// Where the mix is written while it is being made. `Mixdown` removes it when
/// it drops, which is before this function's caller returns.
fn scratch_path() -> PathBuf {
    std::env::temp_dir().join(format!("scorsese-preview-{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// A `Sound` in the given state, without a device anywhere near it.
    ///
    /// Every test here runs on a machine with no sound card — CI is one — so
    /// the state machine is exercised through the channel rather than through
    /// a stream. What cannot be tested without a card is the card; everything
    /// that decides *whether sound is the clock* can be, and that is the part
    /// with the decisions in it.
    fn mixing(sent: Option<Result<Vec<f32>, String>>) -> Sound {
        let (send, mixed) = channel();
        if let Some(answer) = sent {
            send.send(answer).expect("the receiver is still here");
        }
        // Leaked deliberately when nothing was sent: dropping the sender would
        // disconnect the channel and make "still mixing" indistinguishable
        // from "the worker died", which is a distinction `poll` draws.
        std::mem::forget(send);
        Sound::Mixing {
            from: Frames(90),
            since: std::time::Instant::now(),
            mixed,
            rate: RATE,
        }
    }

    /// Only a playing mix is the clock. Everything else answers `None` so the
    /// caller keeps the wall clock — which is what playback did before there
    /// was any sound at all.
    #[test]
    fn nothing_but_a_playing_mix_is_the_clock() {
        assert_eq!(Sound::Silent(None).at(Fps::THIRTY), None);
        assert_eq!(mixing(None).at(Fps::THIRTY), None);
    }

    /// A film with nothing audible in it is silence, not a stream. Opening a
    /// device for an empty buffer would be a voice that plays nothing and
    /// still has to be stopped.
    #[test]
    fn an_empty_mix_becomes_silence_rather_than_a_stream() {
        let mut sound = mixing(Some(Ok(Vec::new())));
        sound.poll();

        assert!(matches!(sound, Sound::Silent(None)));
        assert_eq!(sound.at(Fps::THIRTY), None);
    }

    /// A preview that goes quiet for a reason gives the reason. Silence with
    /// no explanation reads as the sound being broken.
    #[test]
    fn a_mix_that_could_not_be_made_says_why() {
        let mut sound = mixing(Some(Err("ffmpeg is not on PATH".to_owned())));
        sound.poll();

        assert_eq!(sound.trouble(), Some("ffmpeg is not on PATH"));
    }

    /// Still being made is not an answer. The window keeps the wall clock and
    /// asks again on the next repaint, rather than giving up on sound.
    #[test]
    fn a_mix_still_being_made_leaves_the_state_alone() {
        let mut sound = mixing(None);
        sound.poll();

        assert!(matches!(sound, Sound::Mixing { .. }));
    }
}
