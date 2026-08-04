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
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    /// The bytes and what they are.
    pub inline_data: InlineData,
}

/// The bytes of a picture, and their type.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    /// `image/png`, `image/jpeg`.
    pub mime_type: String,
    /// Standard base64, no data-URL prefix.
    pub data: String,
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
    pub fn inline(mime_type: impl Into<String>, base64: impl Into<String>) -> Self {
        Self {
            inline_data: InlineData {
                mime_type: mime_type.into(),
                data: base64.into(),
            },
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
