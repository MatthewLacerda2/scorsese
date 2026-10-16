//! A voice design studio that never spends a cent (#572): three candidates
//! read by ffmpeg's sine, a kept voice named after its candidate, and a
//! forgotten one counted (#1381).

use std::sync::atomic::Ordering;

use scorsese_providers::video::ProviderError;
use scorsese_providers::voices::design::{self, Candidate, DesignError, Studio};
use scorsese_providers::voices::{Voice, VoiceError};

use super::vendors::{Mock, made};

impl Mock {
    pub(super) fn designed(&self) -> usize {
        self.designed.load(Ordering::SeqCst)
    }

    pub(super) fn kept(&self) -> usize {
        self.kept.load(Ordering::SeqCst)
    }

    pub(super) fn forgotten(&self) -> usize {
        self.forgotten.load(Ordering::SeqCst)
    }
}

impl Studio for Mock {
    fn candidates(&self, _: &design::Brief) -> Result<Vec<Candidate>, DesignError> {
        self.designed.fetch_add(1, Ordering::SeqCst);
        Ok((1..=3)
            .map(|n| Candidate {
                generated_voice_id: format!("gen-{n}"),
                sample: made(
                    &format!("sample-{n}.mp3"),
                    &[
                        "-f",
                        "lavfi",
                        "-i",
                        &format!("sine=frequency={}:duration=1", 200 * n),
                    ],
                ),
                seconds: Some(1.0),
            })
            .collect())
    }

    fn keep(
        &self,
        _: &design::Brief,
        chosen: &str,
        name: &str,
        _: &[String],
    ) -> Result<Voice, DesignError> {
        self.kept.fetch_add(1, Ordering::SeqCst);
        Ok(Voice {
            id: format!("voice-of-{chosen}"),
            name: name.to_owned(),
            traits: Vec::new(),
            description: None,
            preview: None,
        })
    }

    fn forget(&self, _: &str) -> Result<(), DesignError> {
        if let Some(why) = &self.refuse_forget {
            return Err(DesignError::Voice(VoiceError::Provider(
                ProviderError::new("mock studio", why),
            )));
        }
        self.forgotten.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "mock studio"
    }
}
