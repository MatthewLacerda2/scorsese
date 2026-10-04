//! Gemini behind the trait: a brief in, a picture out.
//!
//! The whole of this file is translation, kept apart for
//! [`video::veo`](crate::video::VeoProvider)'s reason: a translation spread
//! through the code that makes the call is one nobody can check against the
//! vendor's page.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use scorsese_core::{ImageModel, ImageResolution};

use crate::api::gemini::request::{Create, Input, ResponseFormat};
use crate::api::gemini::{Gemini, Model};
use crate::credentials::Secret;

use super::{Brief, ImageProvider, ProviderError};

/// Gemini's image models, as a provider.
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
}

/// Which model id a brief's model is.
pub(crate) fn model_of(model: ImageModel) -> Model {
    match model {
        ImageModel::Flash => Model::Flash,
        ImageModel::Lite => Model::Lite,
    }
}

/// How a size is spelled on the wire. `512` for the half-K size, which the
/// vendor's page calls "512px (0.5K)"; the rest are the page's own words.
pub(crate) const fn size_of(resolution: ImageResolution) -> &'static str {
    match resolution {
        ImageResolution::K05 => "512",
        ImageResolution::K1 => "1K",
        ImageResolution::K2 => "2K",
        ImageResolution::K4 => "4K",
    }
}

/// The brief as the vendor's request body: the prompt, then each reference.
pub(crate) fn create(brief: &Brief) -> Create {
    let mut input = vec![Input::Text {
        text: brief.prompt.clone(),
    }];
    input.extend(brief.reference_images.iter().map(|still| Input::Image {
        mime_type: still.mime_type.clone(),
        data: STANDARD.encode(&still.bytes),
    }));
    Create {
        model: model_of(brief.request.model).id(),
        input,
        response_format: ResponseFormat {
            kind: "image",
            mime_type: "image/png",
            aspect_ratio: brief.request.aspect.as_str(),
            image_size: size_of(brief.request.size()),
        },
        store: false,
    }
}
