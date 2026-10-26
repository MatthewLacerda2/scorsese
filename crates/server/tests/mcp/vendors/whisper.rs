//! The local Whisper, mocked (#1549).

use std::sync::atomic::Ordering;

use scorsese_core::words::{Word, Words};
use scorsese_providers::transcribe::{Heard, Language, TranscribeError, Transcriber};

use super::Mock;

/// A Whisper that hears "Bom dia." in a WAV, whatever it was sent.
impl Transcriber for Mock {
    fn transcribe(
        &self,
        wav: Vec<u8>,
        language: Option<&Language>,
    ) -> Result<Heard, TranscribeError> {
        self.heard.fetch_add(1, Ordering::SeqCst);
        assert_eq!(&wav[..4], b"RIFF", "the speech is sent as a WAV");
        let said = |text: &str, start: f64, end: f64| Word {
            text: text.into(),
            start,
            end,
        };
        Ok(Heard {
            language: language.map_or("pt", Language::as_str).to_owned(),
            duration: 2.0,
            words: Words {
                words: vec![said("Bom", 0.5, 0.8), said("dia.", 0.8, 1.2)],
            },
        })
    }
}
