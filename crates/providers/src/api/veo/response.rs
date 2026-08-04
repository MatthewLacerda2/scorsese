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

/// The videos an operation produced.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Samples {
    /// One entry per video. Veo returns a single sample per request.
    #[serde(default)]
    pub generated_samples: Vec<Sample>,
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
