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
use crate::api::gemini::response::Interaction;
use crate::api::gemini::{Gemini, MODEL};
use crate::api::http::HttpError;
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
        let reply = self.gemini.create(&create(brief)).map_err(unbilled)?;
        let Some(encoded) = reply.image() else {
            return Err(undrawn(&reply));
        };
        STANDARD
            .decode(encoded)
            .map_err(|_| ProviderError::billed("Gemini", "the picture was not valid base64"))
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
            .map_err(|error| ProviderError::http("Gemini", &error))?;
        if job.name.is_empty() {
            return Err(ProviderError::new("Gemini", "the batch came back unnamed"));
        }
        Ok(job.name)
    }

    fn ask(&self, operation: &str) -> Result<Batch, ProviderError> {
        let job = self
            .gemini
            .batch_status(operation)
            .map_err(|error| ProviderError::http("Gemini", &error))?;
        Ok(batch::read(&job))
    }
}

/// A call that brought back no reply to read: unbilled — except a 2xx whose
/// body does not parse, which Google answered and bills (#1646).
fn unbilled(error: HttpError) -> ProviderError {
    match error {
        HttpError::Unreadable { .. } => ProviderError::billed("Gemini", error.explained(None)),
        error => ProviderError::http("Gemini", &error),
    }
}

/// An interaction that drew nothing: almost always a refusal, and the words
/// are why. Google bills one it completed (#1646); one it reports in any
/// other state, it did not finish.
fn undrawn(reply: &Interaction) -> ProviderError {
    let said = reply.said();
    let status = reply.status.as_deref().unwrap_or("unknown");
    let message = if said.is_empty() {
        format!("the model answered with no picture (status {status})")
    } else {
        said
    };
    if status == "completed" {
        ProviderError::billed("Gemini", message)
    } else {
        ProviderError::new("Gemini", message)
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

    use super::{Brief, create, unbilled, undrawn};
    use crate::api::http::{AT_ITS_LIMIT, HttpError};
    use crate::image::ProviderError;
    use crate::video::Still;

    /// Gemini shares Veo's Google quota, so its 429 is worded as Veo's is
    /// rather than handed to the agent as Google's page of JSON (#1290).
    #[test]
    fn a_429_reads_as_the_sentence_not_the_json() {
        let refused = HttpError::Refused {
            url: String::from("https://generativelanguage.googleapis.com/v1beta/interactions"),
            status: 429,
            body: String::from(r#"{"error":{"code":429,"status":"RESOURCE_EXHAUSTED"}}"#),
        };
        let error = ProviderError::http("Gemini", &refused);
        assert_eq!(error.to_string(), format!("Gemini: {AT_ITS_LIMIT}"));
    }

    /// A completed interaction that drew nothing is billed; one in any other
    /// state, or a call that never answered, is not — but a 2xx that will not
    /// parse was answered (#1646).
    #[test]
    fn only_an_answered_call_that_drew_nothing_is_billed() {
        let reply = |json: &str| serde_json::from_str(json).expect("an interaction");
        let words = r#"{"status": "completed", "steps": [{"type": "model_output",
            "content": [{"type": "text", "text": "I can't draw that."}]}]}"#;
        let completed = undrawn(&reply(words));
        assert!(completed.billed && completed.message == "I can't draw that.");
        assert!(!undrawn(&reply(r#"{"status": "failed", "steps": []}"#)).billed);
        assert!(!undrawn(&reply("{}")).billed);
        let url = String::from("https://generativelanguage.googleapis.com/v1beta/interactions");
        let unreadable = HttpError::Unreadable {
            url: url.clone(),
            message: String::from("expected a value"),
        };
        assert!(unbilled(unreadable).billed);
        let unreachable = HttpError::Refused {
            url,
            status: 503,
            body: String::new(),
        };
        assert!(!unbilled(unreachable).billed);
    }

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
