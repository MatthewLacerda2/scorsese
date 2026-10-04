//! Gemini's image part of the live check: one still, the smallest Google sells.
//!
//! The image client ([`crate::api::gemini`]) was written from Google's
//! image-generation page and had never met the endpoint when it merged (#461).
//! Three of its choices are read off documentation rather than confirmed, and
//! this one call settles all three:
//!
//! - **the endpoint and body** — `POST /v1beta/interactions`, sent exactly as
//!   the product sends it, because the body is built by the product's own
//!   translation of a brief (`image::gemini::create`), not here;
//! - **the `0.5K` spelling** — the page writes the size as "512px (05.K)" and
//!   never as a value, and the client sends `"512"`. A wrong guess is either a
//!   `400` (refused, free) or, worse, ignored — and an ignored size comes back
//!   at the 1K default. So the picture's own header is read: at `1:1` the page's
//!   table gives `512x512` for 0.5K and `1024x1024` for 1K, and anything but
//!   the first is reported as the request's shape having changed;
//! - **where the picture is in the reply** — [`Interaction::image`]'s reading.
//!   If it finds nothing, the recorded body says where the picture really was.
//!
//! No free call stands in for it, for [`veo`](super::veo)'s reason: a request
//! the API is meant to refuse is the vendor's to start accepting. It is the
//! cheapest picture there is — Flash at 0.5K, no references — and it is quoted
//! and held to the ceiling with the rest. The reply is base64 inside JSON, so a
//! recording keeps it whole: that body is what replaces
//! `fixtures/gemini/interaction.json`.

use base64::Engine;
use scorsese_core::{AssetId, ImageAspect, ImageModel, ImageRequest, ImageResolution};

use crate::api::gemini::Gemini;
use crate::api::gemini::request::Create;
use crate::api::gemini::response::Interaction;
use crate::api::http::HttpError;
use crate::api::tap::Tap;
use crate::credentials::Secret;
use crate::image::Brief;
use crate::prices::{self, dollars};

use super::{Step, Verdict, judge};

/// The sentence the still is drawn from: plain, and as unlikely to meet a
/// content filter as [`veo::PROMPT`](super::veo::PROMPT).
pub const PROMPT: &str = "A glass of water on a wooden table, in soft daylight.";

/// The side of the square a 0.5K still is, by Google's table.
pub const SIDE: u32 = 512;

/// The still the check pays for: the cheapest one Gemini sells, square so its
/// expected size is one number.
pub fn still() -> ImageRequest {
    ImageRequest {
        model: ImageModel::Flash,
        resolution: Some(ImageResolution::K05),
        aspect: ImageAspect::Square,
        reference_images: Vec::new(),
    }
}

/// What the still costs, in cents.
pub fn cost() -> u64 {
    prices::image(&still(), PROMPT.chars().count(), 0)
        .expect("Flash Image at 0.5K has a published rate")
        .cents
}

/// The calls, as a quote lists them.
pub(super) fn calls() -> Vec<String> {
    vec![format!(
        "draw one {SIDE}x{SIDE} still (Flash Image, 0.5K, no references) — {}",
        dollars(cost())
    )]
}

/// The body the check sends: the product's translation of the still's brief.
fn body() -> Create {
    crate::image::gemini::create(&Brief {
        id: AssetId::new("live-check"),
        prompt: PROMPT.to_owned(),
        request: still(),
        reference_images: Vec::new(),
    })
}

/// Runs the image part; returns what it found and what it spent, in cents.
pub(super) fn check(key: &Secret, tap: &Tap) -> (Vec<Step>, u64) {
    let step = picture_step(Gemini::new(key).tapped(tap).create(&body()));
    let spent = match step.verdict {
        // A shape change was billed before we failed to read it.
        Verdict::Ok | Verdict::ShapeChanged { .. } => cost(),
        _ => 0,
    };
    (vec![step], spent)
}

/// What the drawing gave back: a PNG, the size asked for.
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
    match judge::png_size(&bytes) {
        None => shape(format!(
            "the picture: expected a PNG, got {}",
            judge::looks_like(&bytes)
        )),
        Some((width, height)) if (width, height) != (SIDE, SIDE) => shape(format!(
            "response_format.image_size: asked for 0.5K ({SIDE}x{SIDE}), got {width}x{height} — the size's spelling was not taken"
        )),
        Some((width, height)) => Step::new(call, Verdict::Ok)
            .noting(format!("{width}x{height} PNG, {} bytes", bytes.len())),
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
                "model": "gemini-3.1-flash-image",
                "input": [{"type": "text", "text": super::PROMPT}],
                "response_format": {
                    "type": "image",
                    "mime_type": "image/png",
                    "aspect_ratio": "1:1",
                    "image_size": "512",
                },
                "store": false,
            })
        );
    }
}
