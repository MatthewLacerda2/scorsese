//! Gemini behind the trait: a brief in, a picture out.
//!
//! The whole of this file is translation, kept apart for
//! [`video::veo`](crate::video::VeoProvider)'s reason: a translation spread
//! through the code that makes the call is one nobody can check against the
//! vendor's page.

mod batch;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use scorsese_core::ImageResolution;

use crate::api::gemini::request::{Create, GenerationConfig, Input, ResponseFormat};
use crate::api::gemini::{Gemini, MODEL};
use crate::credentials::Secret;

use super::{Batch, Brief, ImageProvider, ProviderError};

/// Gemini's Nano Banana 2.1, as a provider.
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    gemini: Gemini,
}

impl GeminiProvider {
    /// One that authenticates with this key — the same key Veo takes.
    pub fn new(key: &Secret) -> Self {
        Self {
            gemini: Gemini::new(key),
        }
    }
}

impl ImageProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "Gemini"
    }

    fn draw(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError> {
        let reply = self
            .gemini
            .create(&create(brief))
            .map_err(|error| ProviderError::new("Gemini", error))?;
        let Some(encoded) = reply.image() else {
            // Answered, and drew nothing: almost always a refusal, and the
            // words are why.
            let said = reply.said();
            let message = if said.is_empty() {
                format!(
                    "the model answered with no picture (status {})",
                    reply.status.as_deref().unwrap_or("unknown")
                )
            } else {
                said
            };
            return Err(ProviderError::new("Gemini", message));
        };
        STANDARD
            .decode(encoded)
            .map_err(|_| ProviderError::new("Gemini", "the picture was not valid base64"))
    }

    fn order(&self, briefs: &[&Brief]) -> Result<String, ProviderError> {
        if briefs.is_empty() {
            return Err(ProviderError::new(
                "Gemini",
                "an empty batch orders nothing",
            ));
        }
        let job = self
            .gemini
            .batch(&batch::create(briefs))
            .map_err(|error| ProviderError::new("Gemini", error))?;
        if job.name.is_empty() {
            return Err(ProviderError::new("Gemini", "the batch came back unnamed"));
        }
        Ok(job.name)
    }

    fn ask(&self, operation: &str) -> Result<Batch, ProviderError> {
        let job = self
            .gemini
            .batch_status(operation)
            .map_err(|error| ProviderError::new("Gemini", error))?;
        Ok(batch::read(&job))
    }
}

/// How a size is spelled on the wire: the page's own words.
pub(crate) const fn size_of(resolution: ImageResolution) -> &'static str {
    match resolution {
        ImageResolution::K1 => "1K",
        ImageResolution::K2 => "2K",
        ImageResolution::K4 => "4K",
    }
}

/// The brief as the vendor's request body: the prompt, then each reference —
/// objects, then characters, which is the order [`Brief::of`] read them in. A
/// thinking level goes only when the brief names one, so a brief that leaves it
/// to the model sends what it always sent. The aspect and the size always go,
/// resolved: left out, Google matches the first reference picture's shape or
/// draws a square, so a reference could quietly set the shape.
pub(crate) fn create(brief: &Brief) -> Create {
    let mut input = vec![Input::Text {
        text: brief.prompt.clone(),
    }];
    input.extend(brief.reference_images.iter().map(|still| Input::Image {
        mime_type: still.mime_type.clone(),
        data: STANDARD.encode(&still.bytes),
    }));
    Create {
        model: MODEL,
        input,
        response_format: ResponseFormat {
            kind: "image",
            mime_type: "image/jpeg",
            aspect_ratio: brief.request.aspect.as_str(),
            image_size: size_of(brief.request.size()),
        },
        generation_config: brief.request.thinking.map(|level| GenerationConfig {
            thinking_level: level.as_str(),
        }),
        store: false,
    }
}

#[cfg(test)]
mod tests {
    use scorsese_core::{AssetId, ImageRequest};

    use super::{Brief, create};
    use crate::video::Still;

    /// Left out, Google matches the first reference picture's shape or draws a
    /// square, so a brief that names neither still sends both, resolved —
    /// a reference can never quietly set the shape (#1308).
    #[test]
    fn the_aspect_and_the_size_are_always_sent() {
        let brief = Brief {
            id: AssetId::new("still"),
            prompt: String::from("a lighthouse"),
            request: ImageRequest::default(),
            reference_images: vec![Still {
                id: AssetId::new("face"),
                mime_type: String::from("image/png"),
                bytes: vec![1, 2, 3],
                digest: String::new(),
            }],
            retired: None,
        };
        let sent = serde_json::to_value(create(&brief)).expect("serialises");
        assert_eq!(sent["model"], "gemini-nano-banana-2.1");
        assert_eq!(sent["response_format"]["aspect_ratio"], "16:9");
        assert_eq!(sent["response_format"]["image_size"], "2K");
    }
}
