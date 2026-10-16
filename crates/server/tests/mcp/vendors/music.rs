//! The mock ElevenLabs Music track: a second of tone (#1264).

use std::sync::atomic::Ordering;

use scorsese_providers::music::{Brief, MusicProvider};
use scorsese_providers::video::ProviderError;

use super::{Mock, made};

impl MusicProvider for Mock {
    fn compose(&self, _: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.composed.fetch_add(1, Ordering::SeqCst);
        Ok(made("track.mp3", &["-f", "lavfi", "-i", "sine=duration=1"]))
    }

    fn name(&self) -> &'static str {
        "mock music"
    }
}
