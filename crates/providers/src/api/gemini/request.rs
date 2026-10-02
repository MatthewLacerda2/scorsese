//! What Gemini is sent, declared field for field as the vendor names it.
//!
//! Snake case throughout, which is the Interactions API's own spelling — unlike
//! Veo's `predictLongRunning`, which is camel case. Checked against the curl
//! examples on Google's image-generation page, 2026-10-02.

use serde::Serialize;

/// The whole POST body of one drawing.
#[derive(Debug, Clone, Serialize)]
pub struct Create {
    /// The model's id.
    pub model: &'static str,
    /// The prompt first, then every reference picture, in order.
    pub input: Vec<Input>,
    /// What to send back: a picture, its type, shape and size.
    pub response_format: ResponseFormat,
    /// Whether Google keeps the interaction to continue it later. Always
    /// `false`: scorsese never continues one — a new brief is a new drawing,
    /// and the vendor keeping a user's prompts and pictures for nothing is a
    /// cost with no benefit.
    pub store: bool,
}

/// One part of the input.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    /// The sentence.
    Text {
        /// What it says.
        text: String,
    },
    /// A picture, inline: base64 in the body, because Google cannot reach a
    /// file on this machine.
    Image {
        /// `image/png`, `image/jpeg`, `image/webp`.
        mime_type: String,
        /// Standard base64, no data-URL prefix.
        data: String,
    },
}

/// What to send back.
#[derive(Debug, Clone, Serialize)]
pub struct ResponseFormat {
    /// Always `image`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// Always `image/png`: lossless, so a still pushed into does not bring its
    /// compression blocks with it.
    pub mime_type: &'static str,
    /// `16:9`, `1:1`, ...
    pub aspect_ratio: &'static str,
    /// `512`, `1K`, `2K` or `4K` — the uppercase K is the vendor's, and a
    /// lowercase one is refused.
    pub image_size: &'static str,
}

/// The wire shape, spelled out, because a shape the compiler accepts and the
/// endpoint does not is only caught by a test that writes the JSON down.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_drawing_with_one_reference_is_the_documented_body() {
        let body = Create {
            model: "gemini-3.1-flash-image",
            input: vec![
                Input::Text {
                    text: String::from("a lighthouse at dusk"),
                },
                Input::Image {
                    mime_type: String::from("image/png"),
                    data: String::from("QUJD"),
                },
            ],
            response_format: ResponseFormat {
                kind: "image",
                mime_type: "image/png",
                aspect_ratio: "16:9",
                image_size: "2K",
            },
            store: false,
        };
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            json!({
                "model": "gemini-3.1-flash-image",
                "input": [
                    {"type": "text", "text": "a lighthouse at dusk"},
                    {"type": "image", "mime_type": "image/png", "data": "QUJD"},
                ],
                "response_format": {
                    "type": "image",
                    "mime_type": "image/png",
                    "aspect_ratio": "16:9",
                    "image_size": "2K",
                },
                "store": false,
            })
        );
    }
}
