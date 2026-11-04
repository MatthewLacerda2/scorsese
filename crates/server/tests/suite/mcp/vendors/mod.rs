//! Vendors that never spend a cent: a Veo, a Gemini — drawing now or in a
//! batch — and an ElevenLabs —
//! speech, music, sound effects and voice design — answering from files ffmpeg made, and
//! counting how often they were asked. Veo is [`video`], the Gemini half [`image`],
//! ElevenLabs' speech [`speech`], its music [`music`] and its sounds
//! [`sound`], and the local
//! Whisper [`whisper`] (#1549).

mod image;
mod music;
mod serve;
mod sound;
mod speech;
mod video;
mod whisper;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use scorsese_render::testing::generate;
use scorsese_server::generations::{Ear, Image, Music, Sound, Speech, Studio, Vendors, Video};

use super::common;

pub(super) use serve::{serve, serve_idle, serve_on, settled, work};

/// What the mock vendors answer, and how often they were asked.
#[derive(Clone, Default)]
pub(super) struct Mock {
    /// A refusal to give when polled, instead of the video.
    pub(super) refuse: Option<String>,
    /// Why Veo no longer knows the ticket, answered when polled (#1196).
    pub(super) lose: Option<String>,
    /// Why a batch stops without drawing, instead of finishing.
    pub(super) stop: Option<String>,
    /// What Gemini or ElevenLabs answers in words, completing the call and
    /// making nothing — billed as the vendor bills it (#1646).
    pub(super) say: Option<String>,
    /// The vendor's own error, instead of a still: a call it did not
    /// complete, so not billed.
    pub(super) fail: Option<String>,
    pub(super) ordered: Arc<AtomicUsize>,
    pub(super) spoken: Arc<AtomicUsize>,
    pub(super) composed: Arc<AtomicUsize>,
    pub(super) sounded: Arc<AtomicUsize>,
    pub(super) submitted: Arc<AtomicUsize>,
    pub(super) drawn: Arc<AtomicUsize>,
    pub(super) designed: Arc<AtomicUsize>,
    pub(super) kept: Arc<AtomicUsize>,
    pub(super) forgotten: Arc<AtomicUsize>,
    pub(super) cloned: Arc<AtomicUsize>,
    /// Why ElevenLabs will not delete a voice, instead of deleting it.
    pub(super) refuse_forget: Option<String>,
    /// How often the Whisper was asked to hear something (#1549).
    pub(super) heard: Arc<AtomicUsize>,
    /// The Whisper container is down.
    pub(super) deaf: bool,
    /// The Whisper listens until it is stopped.
    pub(super) hang: bool,
    /// Stopped while it hangs, the Whisper still lets the request go only
    /// once this is set, as a real one finishes what it began.
    pub(super) late: Option<Arc<AtomicBool>>,
    /// The Whisper answers with this status, and what speaches says with it.
    pub(super) refuses: Option<u16>,
    /// Veo cannot be reached, as on a server with no Gemini key.
    pub(super) keyless_video: bool,
    /// While set, the first line asked for is not answered: a call the
    /// server is stopped in the middle of (#1645).
    pub(super) hold: Arc<AtomicBool>,
}

impl Mock {
    pub(super) fn spoken(&self) -> usize {
        self.spoken.load(Ordering::SeqCst)
    }

    pub(super) fn submitted(&self) -> usize {
        self.submitted.load(Ordering::SeqCst)
    }

    pub(super) fn drawn(&self) -> usize {
        self.drawn.load(Ordering::SeqCst)
    }

    pub(super) fn ordered(&self) -> usize {
        self.ordered.load(Ordering::SeqCst)
    }
}

impl Vendors for Mock {
    fn video(&self) -> Result<Video, String> {
        if self.keyless_video {
            return Err("no Gemini key is set".into());
        }
        Ok(Box::new(self.clone()))
    }

    fn speech(&self) -> Result<Speech, String> {
        Ok(Box::new(self.clone()))
    }

    fn music(&self) -> Result<Music, String> {
        Ok(Box::new(self.clone()))
    }

    fn sound(&self) -> Result<Sound, String> {
        Ok(Box::new(self.clone()))
    }

    fn image(&self) -> Result<Image, String> {
        Ok(Box::new(self.clone()))
    }

    fn studio(&self) -> Result<Studio, String> {
        Ok(Box::new(self.clone()))
    }

    fn whisper(&self) -> Result<Ear, String> {
        Ok(Box::new(self.clone()))
    }
}

/// A file ffmpeg makes from `inputs`, as bytes.
pub(super) fn made(name: &str, inputs: &[&str]) -> Vec<u8> {
    let file = common::scratch::dir("vendor").join(name);
    generate(&common::tools(), &file, inputs);
    std::fs::read(file).expect("the fixture reads")
}
