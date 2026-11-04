//! Where a voice's origin is asked, and the test seam that replaces it.

use std::sync::Arc;

use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::voices::{Catalogue, ElevenLabsVoices};

use super::super::Toolbox;

/// A voice catalogue a call can move to a blocking thread.
pub type Voices = Arc<dyn Catalogue + Send + Sync>;

impl Toolbox {
    /// The same tools asking `voices` where a voice comes from instead of
    /// ElevenLabs — a test's, which needs no key and no network.
    pub fn voiced_from(mut self, voices: Voices) -> Self {
        self.handles.voices = Some(voices);
        self
    }

    /// Where a voice's origin is asked, or why it cannot be.
    pub(in crate::tools) fn catalogue(&self) -> Result<Voices, String> {
        if let Some(voices) = &self.handles.voices {
            return Ok(voices.clone());
        }
        let key = resolve(Provider::ElevenLabs).map_err(|error| error.to_string())?;
        Ok(Arc::new(ElevenLabsVoices::new(&key.secret)))
    }
}
