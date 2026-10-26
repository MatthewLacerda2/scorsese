//! The mock ElevenLabs line: a second of tone, with when each word is said.

use std::sync::atomic::Ordering;

use scorsese_core::words::Words;
use scorsese_providers::speech::{Brief, SpeechProvider, Spoken};
use scorsese_providers::video::ProviderError;

use super::{Mock, made};

impl SpeechProvider for Mock {
    fn speak(&self, _: &Brief) -> Result<Spoken, ProviderError> {
        self.spoken.fetch_add(1, Ordering::SeqCst);
        Ok(Spoken {
            audio: made("line.mp3", &["-f", "lavfi", "-i", "sine=duration=1"]),
            words: Some(timed("Every city has a night editor.")),
        })
    }

    fn name(&self) -> &'static str {
        "mock speech"
    }
}

/// `line`'s words, a character every 30 ms from the start: as ElevenLabs
/// answers with its alignment, folded the same way (#811).
fn timed(line: &str) -> Words {
    Words::from_characters(line.chars().zip(0_u32..).map(|(character, at)| {
        let start = f64::from(at) * 0.03;
        (character, start, start + 0.03)
    }))
}
