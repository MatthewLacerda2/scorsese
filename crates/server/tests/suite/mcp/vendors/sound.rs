//! The mock ElevenLabs sound effect: a second of noise (#1265) — or, told
//! to [`fail`](Mock::fail), the vendor's own error, which is not billed.

use std::sync::atomic::Ordering;

use scorsese_providers::sound::{Brief, SoundProvider};
use scorsese_providers::video::ProviderError;

use super::{Mock, made};

impl SoundProvider for Mock {
    fn make(&self, _: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.sounded.fetch_add(1, Ordering::SeqCst);
        if let Some(why) = &self.fail {
            return Err(ProviderError::new("mock sound", why));
        }
        Ok(made(
            "sound.mp3",
            &["-f", "lavfi", "-i", "anoisesrc=duration=1"],
        ))
    }

    fn name(&self) -> &'static str {
        "mock sound"
    }
}
