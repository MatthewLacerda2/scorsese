//! What Veo sends back, declared as the vendor shapes it.
//!
//! Read permissively on purpose: only the fields scorsese acts on are named,
//! and unknown ones are ignored rather than refused. A request we build is ours
//! to get exactly right, but a reply is somebody else's to change — and a
//! vendor adding a field should not stop a generation we have already paid for
//! from being collected.

use serde::Deserialize;

/// What a submitted generation answers with: a ticket, and nothing else.
#[derive(Debug, Clone, Deserialize)]
pub struct Submitted {
    /// The operation's name, e.g.
    /// `models/veo-3.1-fast-generate-preview/operations/tp33gqfo9wpx`.
    ///
    /// This is the whole of what survives a process exiting. It goes into
    /// `project.json` and is what a later run polls, which is why generating is
    /// not something anyone has to sit and wait for.
    pub name: String,
}

/// What `GET models/{id}` answers with — the part of it the live check reads.
///
/// The endpoint is the whole Gemini API's, so the image models' lookup
/// ([`Gemini::model`](crate::api::gemini::Gemini::model)) answers with it too.
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

/// What polling an operation answers with.
#[derive(Debug, Clone, Deserialize)]
pub struct Operation {
    /// Absent until the work finishes, rather than `false` — which is why this
    /// defaults rather than being required.
    #[serde(default)]
    pub done: bool,
    /// Present once it is done and it worked.
    #[serde(default)]
    pub response: Option<Finished>,
    /// Present once it is done and it did not.
    #[serde(default)]
    pub error: Option<Failure>,
}

/// The successful half of a finished operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finished {
    /// The generated videos, of which there is one.
    pub generate_video_response: Samples,
}

/// The videos an operation produced — or what kept them back.
///
/// The two filter fields are not on Google's Veo page, which says only that a
/// shot can be blocked "because of safety filters or other processing issues
/// with the audio" and that "you will not be charged if your video is blocked
/// from generating" (read 2026-10-11). Their names are the ones Google's own
/// `google-genai` SDK reads off this reply (`_GenerateVideosResponse_from_mldev`,
/// read the same day). No blocked shot has been recorded yet, so
/// `fixtures/veo/blocked.json` is written from that shape (#1109).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Samples {
    /// One entry per video. Veo returns a single sample per request.
    #[serde(default)]
    pub generated_samples: Vec<Sample>,
    /// How many videos the filters kept back. Absent when none were.
    #[serde(default)]
    pub rai_media_filtered_count: u32,
    /// Why, in Google's words — the sentence that says what to rewrite.
    #[serde(default)]
    pub rai_media_filtered_reasons: Vec<String>,
}

/// One generated video.
#[derive(Debug, Clone, Deserialize)]
pub struct Sample {
    /// Where to fetch it from.
    pub video: Video,
}

/// Where a generated video can be downloaded.
#[derive(Debug, Clone, Deserialize)]
pub struct Video {
    /// A Files API link, which needs the same API key as a header — it is not
    /// a public URL, and fetching it without the key gets nothing.
    pub uri: String,
}

/// The failed half of a finished operation.
#[derive(Debug, Clone, Deserialize)]
pub struct Failure {
    /// The vendor's status code.
    #[serde(default)]
    pub code: i32,
    /// The vendor's sentence, which is the part worth showing anybody.
    #[serde(default)]
    pub message: String,
}

impl Operation {
    /// Google's reasons, when its filters kept the video back.
    ///
    /// `Some` — possibly empty, when Google counted a block and gave no reason
    /// — only for a block; `None` for anything else, a video included.
    pub fn blocked(&self) -> Option<&[String]> {
        let samples = &self.response.as_ref()?.generate_video_response;
        let reasons = samples.rai_media_filtered_reasons.as_slice();
        (samples.rai_media_filtered_count > 0 || !reasons.is_empty()).then_some(reasons)
    }

    /// Where to fetch the video, once there is one.
    ///
    /// `None` covers both "not finished" and "finished with nothing in it",
    /// deliberately: neither is something to download, and the caller
    /// distinguishes them by asking [`Operation::done`] and [`Operation::error`]
    /// rather than by reading two absences differently.
    pub fn video_uri(&self) -> Option<&str> {
        Some(
            self.response
                .as_ref()?
                .generate_video_response
                .generated_samples
                .first()?
                .video
                .uri
                .as_str(),
        )
    }
}
