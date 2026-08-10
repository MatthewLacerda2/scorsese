//! Veo behind the trait: a brief in, a ticket out, bytes at the end.
//!
//! The whole of this file is translation. What a request looks like on the wire
//! is [`api::veo`](crate::api::veo)'s business and what the lifecycle is is
//! [`super`]'s; this is the one place that turns a [`Brief`] into the vendor's
//! shape, and it is a separate file for exactly that reason — a translation
//! spread through the code that makes the call is a translation nobody can
//! check against the vendor's documentation.

use crate::api::veo::request::{Generate, Image, Instance, Parameters};
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
            .map_err(|error| ProviderError::new("Veo", error))?;
        Ok(Ticket(submitted.name))
    }

    fn poll(&self, ticket: &Ticket) -> Result<Progress, ProviderError> {
        let operation = self
            .veo
            .poll(&ticket.0)
            .map_err(|error| ProviderError::new("Veo", error))?;
        if !operation.done {
            return Ok(Progress::Waiting);
        }
        if let Some(failure) = operation.error {
            return Ok(Progress::Failed(failure.message));
        }
        match operation.video_uri() {
            Some(uri) => Ok(Progress::Ready(Ready(uri.to_owned()))),
            // Done, no error, and nothing in it. Reported as a failure rather
            // than as still waiting, because waiting for an operation the
            // provider considers finished never ends.
            None => Ok(Progress::Failed(String::from(
                "the operation finished carrying neither a video nor an error",
            ))),
        }
    }

    fn fetch(&self, ready: &Ready) -> Result<Vec<u8>, ProviderError> {
        self.veo
            .fetch(&ready.0)
            .map_err(|error| ProviderError::new("Veo", error))
    }
}

/// Which model id a brief's tier is.
fn model_of(brief: &Brief) -> Model {
    match brief.request.model {
        scorsese_core::VideoModel::Fast => Model::Fast,
        scorsese_core::VideoModel::Lite => Model::Lite,
    }
}

/// The brief as the vendor's request body.
fn generate(brief: &Brief) -> Generate {
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
        },
    }
}

/// A still as the vendor carries one: base64 in the body, because Google has no
/// way to reach a file on this machine.
fn encoded(still: &Still) -> Image {
    Image::encoded(&still.mime_type, base64(&still.bytes))
}

/// How the brief's aspect is spelled on the wire.
fn aspect_of(brief: &Brief) -> &'static str {
    match brief.request.aspect {
        scorsese_core::Aspect::Wide => "16:9",
        scorsese_core::Aspect::Tall => "9:16",
    }
}

/// The alphabet, in the order RFC 4648 §4 numbers it.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` as standard base64 with padding.
///
/// Written here rather than taken as a dependency, for the reason the MCP
/// server gives for its own copy: thirty lines against a fixed specification,
/// and every dependency is one `cargo deny` has to keep clearing.
fn base64(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let bits = group.iter().enumerate().fold(0_u32, |bits, (index, byte)| {
            bits | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            if index <= group.len() {
                encoded.push(char::from(
                    ALPHABET[((bits >> (18 - 6 * index)) & 0b11_1111) as usize],
                ));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}
