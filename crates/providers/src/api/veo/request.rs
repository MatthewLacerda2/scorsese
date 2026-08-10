//! What Veo is sent, declared field for field as the vendor names it.
//!
//! Nothing here is built by hand at a call site. Every request scorsese makes
//! is one of these values, serialised — so this file and Google's REST page can
//! be read side by side and checked off against each other, which is the only
//! practical way to keep a hand-written client honest about somebody else's
//! API.

use serde::Serialize;

/// The whole POST body of a generation request.
///
/// The odd `instances`/`parameters` split is the vendor's, not ours: it is
/// inherited from Vertex's prediction API, where one call could carry several
/// inputs. Veo takes exactly one, and mirroring the shape rather than
/// flattening it is the point of this file.
#[derive(Debug, Clone, Serialize)]
pub struct Generate {
    /// Always exactly one, however plural the field name is.
    pub instances: [Instance; 1],
    /// How to generate it, as opposed to what.
    pub parameters: Parameters,
}

/// What to generate: the sentence, and any stills it is built from.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// The sentence.
    pub prompt: String,
    /// The frame to open on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    /// The frame to end on. Only meaningful beside `image`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_frame: Option<Image>,
    /// Stills of a subject that should keep looking like itself.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reference_images: Vec<ReferenceImage>,
}

/// A picture, inline in the request.
///
/// Base64 in the body rather than a URL, because the vendor has no way to
/// reach a file on this machine — which means the bytes of every still a brief
/// names are read and encoded at submit time, and a request carrying three
/// reference images carries three whole images.
///
/// The bytes sit **directly on the image object** — not wrapped in the
/// `inlineData` content part that Gemini's `generateContent` uses. The
/// vendor's own curl examples show `inlineData` here, and the live endpoint
/// refuses it: a real submission on 2026-08-09 got 400 *"`inlineData` isn't
/// supported by this model"*. These field names are what Google's own
/// `google-genai` SDK puts on the wire for this endpoint, and they are the
/// ones the API accepts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    /// Standard base64, no data-URL prefix.
    pub bytes_base64_encoded: String,
    /// `image/png`, `image/jpeg`.
    pub mime_type: String,
}

/// A still, and what the model should take from it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceImage {
    /// The picture.
    pub image: Image,
    /// What it is for. `asset` — the subject to preserve.
    ///
    /// A `style` type exists on older models and is not offered on 3.1, so it
    /// is not modelled: a field that can only hold one value is honest about
    /// the API rather than pretending at a choice.
    pub reference_type: &'static str,
}

/// How to generate, as opposed to what.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    /// `16:9` or `9:16`.
    pub aspect_ratio: String,
    /// 4, 6 or 8.
    pub duration_seconds: u32,
    /// `720p` or `1080p`.
    pub resolution: String,
}

impl Image {
    /// A picture from its bytes and its type.
    pub fn encoded(mime_type: impl Into<String>, base64: impl Into<String>) -> Self {
        Self {
            bytes_base64_encoded: base64.into(),
            mime_type: mime_type.into(),
        }
    }

    /// The same picture, as something to keep a subject consistent with.
    pub fn as_asset(self) -> ReferenceImage {
        ReferenceImage {
            image: self,
            reference_type: "asset",
        }
    }
}

/// The wire shape, pinned byte for byte against what the endpoint accepts.
///
/// These literals are the contract: the 2026-08-09 incident was a shape the
/// compiler was happy with and the API was not, which only a test that spells
/// the JSON out can catch.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A one-pixel stand-in; the endpoint never sees these tests.
    fn still() -> Image {
        Image::encoded("image/png", "QUJD")
    }

    #[test]
    fn images_carry_their_bytes_directly_not_as_inline_data() {
        let body = Generate {
            instances: [Instance {
                prompt: String::from("a butterfly"),
                image: Some(still()),
                last_frame: Some(still()),
                reference_images: vec![still().as_asset()],
            }],
            parameters: Parameters {
                aspect_ratio: String::from("16:9"),
                duration_seconds: 8,
                resolution: String::from("720p"),
            },
        };
        let picture = json!({"bytesBase64Encoded": "QUJD", "mimeType": "image/png"});
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            json!({
                "instances": [{
                    "prompt": "a butterfly",
                    "image": picture,
                    "lastFrame": picture,
                    "referenceImages": [{"image": picture, "referenceType": "asset"}],
                }],
                "parameters": {
                    "aspectRatio": "16:9",
                    "durationSeconds": 8,
                    "resolution": "720p",
                },
            })
        );
    }

    #[test]
    fn a_prompt_alone_sends_no_image_fields_at_all() {
        let instance = Instance {
            prompt: String::from("a butterfly"),
            ..Instance::default()
        };
        assert_eq!(
            serde_json::to_value(&instance).unwrap(),
            json!({"prompt": "a butterfly"})
        );
    }
}
