//! What Google's two APIs here share: Gemini's image models ([`gemini`]) and
//! Veo ([`veo`]) are one API, the Gemini API, behind one root, one key and one
//! header — so those, and the model lookup both answer with, are written once.
//!
//! [`gemini`]: crate::api::gemini
//! [`veo`]: crate::api::veo

use serde::Deserialize;

use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// The API's root, which every Gemini and Veo endpoint hangs off.
pub(crate) const BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// The header Google reads the key from — the same key for pictures and video.
const KEY_HEADER: &str = "x-goog-api-key";

/// A caller that authenticates with this key.
pub(crate) fn caller(key: &Secret) -> Caller {
    Caller::new(KEY_HEADER, key)
}

/// What the API says about the model `id`: its full name, and the methods it
/// can be called with. Free — nothing is made.
pub(crate) fn model(caller: &Caller, id: &str) -> Result<ModelInfo, HttpError> {
    caller.get(&format!("{BASE}/models/{id}"))
}

/// What `GET models/{id}` answers with — the part of it the live check reads.
///
/// Read permissively, for [`veo::response`](crate::api::veo::response)'s
/// reason: a reply is somebody else's to change.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// `models/` and the id, e.g. `models/veo-3.1-fast-generate-preview`.
    pub name: String,
    /// How it may be called. A video model lists `predictLongRunning`, which
    /// is the one method scorsese submits with; an image model lists
    /// `generateContent` and `batchGenerateContent`.
    #[serde(default)]
    pub supported_generation_methods: Vec<String>,
}
