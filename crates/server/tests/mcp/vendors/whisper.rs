//! The local Whisper, mocked (#1549).

use std::sync::atomic::Ordering;
use std::time::Duration;

use scorsese_core::words::{Word, Words};
use scorsese_providers::transcribe::{Heard, Language, TranscribeError, Transcriber};

use super::Mock;

/// A Whisper that, unless it is deaf or hangs, hears "Bom dia." in a WAV,
/// whatever it was sent.
impl Transcriber for Mock {
    fn transcribe(
        &self,
        wav: Vec<u8>,
        language: Option<&Language>,
        stop: &dyn Fn() -> bool,
    ) -> Result<Heard, TranscribeError> {
        self.heard.fetch_add(1, Ordering::SeqCst);
        if self.hang {
            // Listens until it is stopped, as a long recording keeps it.
            let asked = std::time::Instant::now();
            while !stop() {
                assert!(asked.elapsed() < Duration::from_secs(20), "never stopped");
                std::thread::sleep(Duration::from_millis(10));
            }
            return Err(TranscribeError::Stopped);
        }
        if self.deaf {
            return Err(TranscribeError::NotRunning {
                address: "http://whisper:8000".into(),
                why: "connection refused".into(),
            });
        }
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
