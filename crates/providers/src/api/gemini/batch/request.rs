//! What a batch job is sent: `generateContent` requests, inline, each with a
//! key to find its answer by.
//!
//! Camel case throughout, the REST spelling of `generateContent` — unlike the
//! Interactions body in [`super::super::request`], which is snake case.
//! Checked against the batch page's inline example and the image-generation
//! page's `generationConfig.imageConfig`, 2026-10-08.

use serde::Serialize;

/// The whole POST body of one job.
#[derive(Debug, Clone, Serialize)]
pub struct Create {
    /// The job.
    pub batch: Batch,
}

/// One job.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    /// A name a person reads in Google's console; never read back.
    pub display_name: String,
    /// Where the requests are.
    pub input_config: InputConfig,
}

/// Where the requests are: inline, always — see [`super::INLINE_LIMIT`].
#[derive(Debug, Clone, Serialize)]
pub struct InputConfig {
    /// The requests.
    pub requests: Requests,
}

/// The inline requests, in the page's doubly nested spelling.
#[derive(Debug, Clone, Serialize)]
pub struct Requests {
    /// Each one.
    pub requests: Vec<Keyed>,
}

/// One request and the key its answer comes back under.
#[derive(Debug, Clone, Serialize)]
pub struct Keyed {
    /// The drawing.
    pub request: Request,
    /// The key.
    pub metadata: Metadata,
}

/// What a request is known by in the answer.
#[derive(Debug, Clone, Serialize)]
pub struct Metadata {
    /// Free text, handed back as it was sent.
    pub key: String,
}

/// One `generateContent` request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// One turn: the prompt, then every reference picture.
    pub contents: Vec<Content>,
    /// What to draw and how.
    pub generation_config: GenerationConfig,
}

/// One turn of the conversation.
#[derive(Debug, Clone, Serialize)]
pub struct Content {
    /// Words and pictures, in order.
    pub parts: Vec<Part>,
}

/// One piece of a turn.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Part {
    /// The sentence.
    Text {
        /// What it says.
        text: String,
    },
    /// A picture, inline.
    Image {
        /// The picture.
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
}

/// A picture, base64.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    /// `image/png`, `image/jpeg`, `image/webp`.
    pub mime_type: String,
    /// Standard base64.
    pub data: String,
}

/// What to send back.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Always `["IMAGE"]`: a picture, and no words beside it.
    pub response_modalities: Vec<&'static str>,
    /// Its shape and size.
    pub image_config: ImageConfig,
    /// How the model works before it draws; absent leaves it to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

/// A picture's shape and size.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    /// `16:9`, `1:1`, ...
    pub aspect_ratio: &'static str,
    /// `512`, `1K`, `2K` or `4K`.
    pub image_size: &'static str,
}

/// How the model thinks.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    /// `minimal`, `medium` or `high`.
    pub thinking_level: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_job_of_one_still_with_a_reference_is_the_documented_body() {
        let body = Create {
            batch: Batch {
                display_name: String::from("scorsese stills"),
                input_config: InputConfig {
                    requests: Requests {
                        requests: vec![Keyed {
                            request: Request {
                                contents: vec![Content {
                                    parts: vec![
                                        Part::Text {
                                            text: String::from("a lighthouse"),
                                        },
                                        Part::Image {
                                            inline_data: InlineData {
                                                mime_type: String::from("image/png"),
                                                data: String::from("QUJD"),
                                            },
                                        },
                                    ],
                                }],
                                generation_config: GenerationConfig {
                                    response_modalities: vec!["IMAGE"],
                                    image_config: ImageConfig {
                                        aspect_ratio: "16:9",
                                        image_size: "2K",
                                    },
                                    thinking_config: Some(ThinkingConfig {
                                        thinking_level: "high",
                                    }),
                                },
                            },
                            metadata: Metadata {
                                key: String::from("poster-abc"),
                            },
                        }],
                    },
                },
            },
        };
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            json!({"batch": {
                "displayName": "scorsese stills",
                "inputConfig": {"requests": {"requests": [{
                    "request": {
                        "contents": [{"parts": [
                            {"text": "a lighthouse"},
                            {"inlineData": {"mimeType": "image/png", "data": "QUJD"}},
                        ]}],
                        "generationConfig": {
                            "responseModalities": ["IMAGE"],
                            "imageConfig": {"aspectRatio": "16:9", "imageSize": "2K"},
                            "thinkingConfig": {"thinkingLevel": "high"},
                        },
                    },
                    "metadata": {"key": "poster-abc"},
                }]}},
            }})
        );
    }
}
