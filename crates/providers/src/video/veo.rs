//! Veo behind the trait: a brief in, a ticket out, bytes at the end.
//!
//! The whole of this file is translation. What a request looks like on the wire
//! is [`api::veo`](crate::api::veo)'s business and what the lifecycle is is
//! [`super`]'s; this is the one place that turns a [`Brief`] into the vendor's
//! shape, and it is a separate file for exactly that reason — a translation
//! spread through the code that makes the call is a translation nobody can
//! check against the vendor's documentation.

use base64::Engine;

use crate::api::http::HttpError;
use crate::api::veo::request::{Generate, Image, Instance, Parameters};
use crate::api::veo::response::Operation;
use crate::api::veo::{Model, Veo};
use crate::credentials::Secret;

use super::brief::Still;
use super::provider::ProviderError;
use super::{Brief, Progress, Ready, Ticket, VideoProvider};

/// Veo, as a provider.
#[derive(Debug, Clone)]
pub struct VeoProvider {
    veo: Veo,
}

impl VeoProvider {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self { veo: Veo::new(key) }
    }
}

impl VideoProvider for VeoProvider {
    fn name(&self) -> &'static str {
        "Veo"
    }

    fn submit(&self, brief: &Brief) -> Result<Ticket, ProviderError> {
        let submitted = self
            .veo
            .submit(model_of(brief), &generate(brief))
            .map_err(|error| ProviderError::http("Veo", &error))?;
        Ok(Ticket(submitted.name))
    }

    fn poll(&self, ticket: &Ticket) -> Result<Progress, ProviderError> {
        let operation = self.veo.poll(&ticket.0).map_err(unpollable)?;
        Ok(progress(&operation))
    }

    fn fetch(&self, ready: &Ready) -> Result<Vec<u8>, ProviderError> {
        self.veo
            .fetch(&ready.0)
            .map_err(|error| ProviderError::http("Veo", &error))
    }
}

/// What a polled operation means for the shot.
fn progress(operation: &Operation) -> Progress {
    if !operation.done {
        return Progress::Waiting;
    }
    if let Some(failure) = &operation.error {
        return Progress::Failed(failure.message.clone());
    }
    if let Some(uri) = operation.video_uri() {
        return Progress::Ready(Ready(uri.to_owned()));
    }
    match operation.blocked() {
        Some(reasons) => Progress::Failed(blocked(reasons)),
        // Done, no error, and nothing in it. Reported as a failure rather
        // than as still waiting, because waiting for an operation the
        // provider considers finished never ends.
        None => Progress::Failed(String::from(
            "the operation finished carrying neither a video nor an error",
        )),
    }
}

/// A shot Veo's filters kept back, in words an agent can act on (#1109): that
/// it cost nothing, and Google's reason, which is what to rewrite.
///
/// "Not charged" is Google's own promise for a blocked shot, on its Veo page —
/// and a refused shot is recorded as spending nothing here either, so the
/// sentence and the ledger agree.
pub(crate) fn blocked(reasons: &[String]) -> String {
    let why = if reasons.is_empty() {
        String::from("Google gave no reason")
    } else {
        reasons.join(" / ")
    };
    format!(
        "blocked by Veo's safety filters, and not charged: {why} — rewrite the prompt, its audio cues included, and generate again"
    )
}

/// A poll Veo would not answer, in words.
///
/// **403 and 404 are permanent**: Google's error model answers `NOT_FOUND` for
/// an operation it does not have and `PERMISSION_DENIED` for one this key may
/// not see — a rotated key, another Google account. Asking again cannot change
/// either, so the ticket is [lost](ProviderError::lost) (#1106). Anything else
/// — a 5xx, a 429, no network — is worth asking again later.
fn unpollable(error: HttpError) -> ProviderError {
    match error {
        HttpError::Refused {
            status: status @ (403 | 404),
            ..
        } => ProviderError::lost(
            "Veo",
            format!(
                "Veo no longer answers for this shot's ticket ({status}) — it does not have \
                 it, or this key may not see it"
            ),
        ),
        other => ProviderError::http("Veo", &other),
    }
}

/// Which model id a brief's tier is. The live check ([`crate::live`]) submits
/// through this too, so it tests the product's translation, not its own.
pub(crate) fn model_of(brief: &Brief) -> Model {
    match brief.request.model {
        scorsese_core::VideoModel::Standard => Model::Standard,
        scorsese_core::VideoModel::Fast => Model::Fast,
        scorsese_core::VideoModel::Lite => Model::Lite,
    }
}

/// The brief as the vendor's request body.
pub(crate) fn generate(brief: &Brief) -> Generate {
    Generate {
        instances: [Instance {
            prompt: brief.prompt.clone(),
            image: brief.first_image.as_ref().map(encoded),
            last_frame: brief.last_image.as_ref().map(encoded),
            reference_images: brief
                .reference_images
                .iter()
                .map(|still| encoded(still).as_asset())
                .collect(),
        }],
        parameters: Parameters {
            aspect_ratio: aspect_of(brief).to_owned(),
            duration_seconds: brief.request.seconds.get(),
            resolution: brief.request.resolution.as_str().to_owned(),
            seed: brief.request.seed,
        },
    }
}

/// A still as the vendor carries one: base64 in the body, because Google has no
/// way to reach a file on this machine.
fn encoded(still: &Still) -> Image {
    Image::encoded(
        &still.mime_type,
        base64::engine::general_purpose::STANDARD.encode(&still.bytes),
    )
}

/// How the brief's aspect is spelled on the wire.
fn aspect_of(brief: &Brief) -> &'static str {
    match brief.request.aspect {
        scorsese_core::Aspect::Wide => "16:9",
        scorsese_core::Aspect::Tall => "9:16",
    }
}

/// How a poll is read: which refusals lose the ticket, and a reply read as the
/// shot's progress, from the fixtures the live check reads too.
#[cfg(test)]
mod tests {
    use super::*;

    fn poll_refused(status: u16) -> ProviderError {
        unpollable(HttpError::Refused {
            url: String::from("https://veo/operations/1"),
            status,
            body: String::new(),
        })
    }

    #[test]
    fn only_a_403_or_a_404_loses_a_ticket() {
        assert!(poll_refused(403).lost && poll_refused(404).lost);
        assert!(poll_refused(404).message.contains("(404)"));
        for status in [400, 429, 500, 503] {
            assert!(!poll_refused(status).lost, "{status} is worth asking again");
        }
    }

    fn polled(fixture: &str) -> Progress {
        let path = format!("{}/fixtures/veo/{fixture}", env!("CARGO_MANIFEST_DIR"));
        let body = std::fs::read_to_string(&path).expect("the fixture");
        progress(&serde_json::from_str(&body).expect("an operation"))
    }

    #[test]
    fn a_blocked_shot_says_it_was_not_charged_and_why() {
        let Progress::Failed(said) = polled("blocked.json") else {
            panic!("a blocked shot is a refusal");
        };
        assert!(said.contains("not charged"), "{said}");
        assert!(said.contains("issue with the audio"), "{said}");
    }

    #[test]
    fn a_block_without_a_reason_still_says_so() {
        let operation = serde_json::from_str(
            r#"{"done": true, "response": {"generateVideoResponse": {"raiMediaFilteredCount": 1}}}"#,
        )
        .expect("an operation");
        let Progress::Failed(said) = progress(&operation) else {
            panic!("a blocked shot is a refusal");
        };
        assert!(
            said.contains("not charged: Google gave no reason"),
            "{said}"
        );
    }

    #[test]
    fn the_other_answers_read_as_before() {
        assert_eq!(polled("running.json"), Progress::Waiting);
        assert!(matches!(polled("done.json"), Progress::Ready(_)));
        assert_eq!(
            polled("failed.json"),
            Progress::Failed(String::from("The prompt could not be submitted."))
        );
    }
}
