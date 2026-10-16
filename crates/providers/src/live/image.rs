//! Gemini's image part of the live check: the model's id, free, then one
//! still, the smallest Google sells.
//!
//! # The free part
//!
//! `GET models/gemini-nano-banana-2.1`, the one image model scorsese uses
//! (#1308), as [`veo`](super::veo) does for its tiers (#948). It proves the id
//! is still served, and with both methods the product calls:
//! `generateContent`, the method a [batch](crate::api::gemini::batch) is made
//! of, and `batchGenerateContent`, which creates one. A renamed or retired
//! model is the change that fails every still on the web at once, and it costs
//! nothing to catch — the model's id once merged unconfirmed (#893).
//!
//! # The paid part
//!
//! The image client ([`crate::api::gemini`]) was written from Google's
//! image-generation page and had never met the endpoint when it merged (#461).
//! Three of its choices are read off documentation rather than confirmed, and
//! this one call settles all three:
//!
//! - **the endpoint and body** — `POST /v1beta/interactions`, sent exactly as
//!   the product sends it, because the body is built by the product's own
//!   translation of a brief (`image::gemini::create`), not here;
//! - **that the size and shape are taken** — an ignored `image_size` or
//!   `aspect_ratio` comes back at whatever the model defaults to, billed as
//!   what was asked. So the picture's own header is read: at `1:1` the page's
//!   table gives `1024x1024` for 1K, and anything else is reported as the
//!   request's shape having changed;
//! - **where the picture is in the reply** — [`Interaction::image`]'s reading.
//!   If it finds nothing, the recorded body says where the picture really was.
//!
//! No free call stands in for it, for [`veo`](super::veo)'s reason: a request
//! the API is meant to refuse is the vendor's to start accepting. It is the
//! smallest picture there is — 2.1 at 1K, no references, $0.0336 — and it is
//! quoted and held to the ceiling with the rest. The reply is base64 inside
//! JSON, so a recording keeps it whole: that body is what replaces
//! `fixtures/gemini/interaction.json`.

use base64::Engine;
use scorsese_core::{AssetId, ImageAspect, ImageRequest, ImageResolution};

use crate::api::gemini::request::Create;
use crate::api::gemini::response::Interaction;
use crate::api::gemini::{Gemini, MODEL};
use crate::api::google::ModelInfo;
use crate::api::http::HttpError;
use crate::api::tap::Tap;
use crate::credentials::Secret;
use crate::image::Brief;
use crate::prices::{self, dollars, from_cents};

use super::{Step, Verdict, judge};

/// The sentence the still is drawn from: plain, and as unlikely to meet a
/// content filter as [`veo::PROMPT`](super::veo::PROMPT).
pub const PROMPT: &str = "A glass of water on a wooden table, in soft daylight.";

/// The methods the image model must list: the one a batch's requests are in,
/// and the one that creates the batch.
const METHODS: [&str; 2] = ["generateContent", "batchGenerateContent"];

/// The side of the square a 1K still is, by Google's table.
pub const SIDE: u32 = 1024;

/// The still the check pays for: the cheapest one Gemini sells, square so its
/// expected size is one number.
pub fn still() -> ImageRequest {
    ImageRequest {
        resolution: Some(ImageResolution::K1),
        aspect: ImageAspect::Square,
        ..ImageRequest::default()
    }
}

/// What the still costs, in cents.
pub fn cost() -> u64 {
    prices::image(&still(), PROMPT.chars().count(), 0)
        .expect("Nano Banana 2.1 at 1K has a published rate")
        .cents
}

/// The calls, as a quote lists them.
pub(super) fn calls() -> Vec<String> {
    vec![
        format!("GET models/{MODEL} — free"),
        format!(
            "draw one {SIDE}x{SIDE} still (Nano Banana 2.1, 1K, no references) — {}",
            dollars(from_cents(cost()), 2)
        ),
    ]
}

/// The body the check sends: the product's translation of the still's brief.
fn body() -> Create {
    crate::image::gemini::create(&Brief {
        id: AssetId::new("live-check"),
        prompt: PROMPT.to_owned(),
        request: still(),
        reference_images: Vec::new(),
        retired: None,
    })
}

/// Runs the image part; returns what it found and what it spent, in cents.
pub(super) fn check(key: &Secret, tap: &Tap) -> (Vec<Step>, u64) {
    let gemini = Gemini::new(key).tapped(tap);
    let mut steps = vec![model_step(gemini.model())];
    let step = picture_step(gemini.create(&body()));
    let spent = match step.verdict {
        // A shape change was billed before we failed to read it.
        Verdict::Ok | Verdict::ShapeChanged { .. } => cost(),
        _ => 0,
    };
    steps.push(step);
    (steps, spent)
}

/// What `GET models/{id}` said about the image model.
pub fn model_step(answer: Result<ModelInfo, HttpError>) -> Step {
    let verdict = judge::model(MODEL, &METHODS, answer);
    Step::new(format!("GET models/{MODEL}"), verdict)
}

/// What the drawing gave back: a JPEG, the size asked for.
pub fn picture_step(answer: Result<Interaction, HttpError>) -> Step {
    let call = "POST interactions";
    let shape = |field: String| Step::new(call, Verdict::ShapeChanged { field });
    let reply = match answer {
        Err(error) => return Step::new(call, judge::gemini(&error)),
        Ok(reply) => reply,
    };
    let Some(encoded) = reply.image() else {
        let said = reply.said();
        if !said.is_empty() {
            // Words and no picture is how the model declines.
            return Step::new(call, Verdict::Refused { said });
        }
        return shape(format!(
            "steps[].content[].data / output_image.data: no picture in the reply (status {})",
            reply.status.as_deref().unwrap_or("absent")
        ));
    };
    let Some(bytes) = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()
    else {
        return shape(String::from("the picture's data: not base64"));
    };
    match judge::jpeg_size(&bytes) {
        None => shape(format!(
            "the picture: expected a JPEG, got {}",
            judge::looks_like(&bytes)
        )),
        Some((width, height)) if (width, height) != (SIDE, SIDE) => shape(format!(
            "response_format.image_size: asked for 1K ({SIDE}x{SIDE}), got {width}x{height} — the size or the shape was not taken"
        )),
        Some((width, height)) => Step::new(call, Verdict::Ok)
            .noting(format!("{width}x{height} JPEG, {} bytes", bytes.len())),
    }
}

/// The request is the product's, so this pins what the product sends for the
/// cheapest still — the body a person's paid run will put to the vendor.
#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn the_check_sends_the_smallest_still_through_the_products_translation() {
        assert_eq!(
            serde_json::to_value(super::body()).unwrap(),
            json!({
                "model": "gemini-nano-banana-2.1",
                "input": [{"type": "text", "text": super::PROMPT}],
                "response_format": {
                    "type": "image",
                    "mime_type": "image/jpeg",
                    "aspect_ratio": "1:1",
                    "image_size": "1K",
                },
                "store": false,
            })
        );
    }
}
