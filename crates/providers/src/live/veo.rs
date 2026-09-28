//! Veo's part of the live check: free by default, one real shot on request.
//!
//! # Why the shot is opt-in rather than faked
//!
//! Every other call in the check costs a cent or two. The cheapest thing Veo
//! sells — four seconds of Lite at 720p — is [`shot`]'s twenty cents, ten to a
//! hundred times the rest of the check together, and it takes minutes. A check
//! meant to be run before every deploy should be one nobody hesitates over, so
//! by default Veo gets the calls that cost nothing: `GET models/{id}` for each
//! tier scorsese offers, which proves the key is accepted and that each
//! `-preview` id is still served with `predictLongRunning` — the two changes
//! most likely to break a generation, and the ones Google has form for.
//!
//! The obvious middle road — a submit the API refuses, to check the endpoint
//! without paying — was weighed and turned down. Whether a malformed request
//! is refused is the vendor's decision, not ours: the day Veo starts clamping
//! an out-of-range duration instead of rejecting it, the "free" probe
//! generates a shot nobody quoted, and a check that can spend without asking
//! breaks the one rule every spend here obeys. It would also verify nothing
//! the model lookup does not, because a refusal carries no operation.
//!
//! What only a real generation can verify is the shape of the operation it
//! returns — submitted, running, done — and the download behind the key. None
//! of those has a captured body yet (`fixtures/veo/` is hand-written), so the
//! shot is also how that gets fixed: it is quoted, held to the ceiling, and
//! run through the product's own translation of a brief
//! ([`VeoProvider`](crate::video::VeoProvider)'s), not a body written here.

use std::time::{Duration, Instant};

use scorsese_core::{Aspect, AssetId, ClipSeconds, VideoModel, VideoRequest, VideoResolution};

use crate::api::http::HttpError;
use crate::api::tap::Tap;
use crate::api::veo::response::{ModelInfo, Operation, Submitted};
use crate::api::veo::{Model, Veo};
use crate::credentials::Secret;
use crate::prices::{self, dollars};
use crate::video::{Brief, POLL_EVERY};

use super::{Options, Step, Verdict, judge};

/// The sentence the shot is generated from: plain, still, and as unlikely to
/// meet a content filter as a sentence can be.
pub const PROMPT: &str = "A glass of water on a wooden table beside a window, in soft daylight.";

/// The method a video model must list for scorsese to submit to it.
const METHOD: &str = "predictLongRunning";

/// The shot `--include-veo` pays for: the cheapest one Veo sells.
pub fn shot() -> VideoRequest {
    VideoRequest {
        model: VideoModel::Lite,
        resolution: VideoResolution::P720,
        seconds: ClipSeconds::Four,
        aspect: Aspect::Wide,
        ..VideoRequest::default()
    }
}

/// What the Veo part of the check costs, in cents.
pub fn cost(options: &Options) -> u64 {
    if !options.include_veo {
        return 0;
    }
    prices::estimate(&shot())
        .expect("Veo 3.1 Lite at 720p has a published rate")
        .cents
}

/// The calls, as a quote lists them.
pub(super) fn calls(options: &Options) -> Vec<String> {
    let ids: Vec<String> = Model::ALL
        .iter()
        .map(|m| format!("models/{}", m.id()))
        .collect();
    let shot = if options.include_veo {
        format!(
            "one 4-second 720p Veo 3.1 Lite shot — submit, poll, download — {}",
            dollars(cost(options))
        )
    } else {
        format!(
            "no shot — a real one is opt-in, at {}",
            dollars(cost(&Options {
                include_veo: true,
                ..*options
            }))
        )
    };
    vec![format!("GET {} — free", ids.join(", ")), shot]
}

/// Runs the Veo part; returns what it found and what it spent, in cents.
pub(super) fn check(
    key: &Secret,
    tap: &Tap,
    options: &Options,
    on: &mut dyn FnMut(&str),
) -> (Vec<Step>, u64) {
    let veo = Veo::new(key).tapped(tap);
    let mut steps: Vec<Step> = Model::ALL
        .iter()
        .map(|model| model_step(*model, veo.model(*model)))
        .collect();
    if !options.include_veo {
        steps.push(Step::new(
            "a real shot",
            Verdict::Skipped {
                why: format!(
                    "opt-in: it costs {}",
                    dollars(cost(&Options {
                        include_veo: true,
                        ..*options
                    }))
                ),
            },
        ));
        return (steps, 0);
    }
    let brief = Brief {
        id: AssetId::new("live-check"),
        prompt: PROMPT.to_owned(),
        request: shot(),
        first_image: None,
        last_image: None,
        reference_images: Vec::new(),
    };
    let submitted = veo.submit(
        crate::video::veo::model_of(&brief),
        &crate::video::veo::generate(&brief),
    );
    let (step, ticket) = submitted_step(submitted);
    steps.push(step);
    let Some(ticket) = ticket else {
        return (steps, 0);
    };
    let (step, uri) = wait(&veo, &ticket, options.veo_patience, on);
    steps.push(step);
    if let Some(uri) = uri {
        steps.push(video_step(veo.fetch(&uri)));
    }
    (steps, cost(options))
}

/// Polls until the operation finishes or `patience` runs out.
fn wait(
    veo: &Veo,
    ticket: &str,
    patience: Duration,
    on: &mut dyn FnMut(&str),
) -> (Step, Option<String>) {
    let started = Instant::now();
    loop {
        let polled = veo.poll(ticket);
        if let Some(done) = operation_step(polled) {
            return done;
        }
        if started.elapsed() >= patience {
            let said = format!(
                "still generating after {}s — operation {ticket}; its finished shape went unchecked",
                patience.as_secs()
            );
            return (
                Step::new("GET operation", Verdict::Unfinished { said }),
                None,
            );
        }
        on(&format!(
            "Veo is still generating ({}s so far)…",
            started.elapsed().as_secs()
        ));
        std::thread::sleep(POLL_EVERY);
    }
}

/// What `GET models/{id}` said about one tier.
pub fn model_step(model: Model, answer: Result<ModelInfo, HttpError>) -> Step {
    let call = format!("GET models/{}", model.id());
    let expected = format!("models/{}", model.id());
    let verdict = match answer {
        Err(error) => judge::gemini(&error),
        Ok(info) if info.name != expected => Verdict::ShapeChanged {
            field: format!("name: expected {expected}, got {:?}", info.name),
        },
        Ok(info)
            if !info
                .supported_generation_methods
                .iter()
                .any(|m| m == METHOD) =>
        {
            Verdict::ShapeChanged {
                field: format!(
                    "supportedGenerationMethods has no {METHOD}: {:?}",
                    info.supported_generation_methods
                ),
            }
        }
        Ok(_) => Verdict::Ok,
    };
    Step::new(call, verdict)
}

/// What the submit said, and the ticket to poll if it gave one.
pub fn submitted_step(answer: Result<Submitted, HttpError>) -> (Step, Option<String>) {
    let call = "POST :predictLongRunning";
    match answer {
        Err(error) => (Step::new(call, judge::gemini(&error)), None),
        Ok(submitted) if !submitted.name.contains("/operations/") => {
            let field = format!("name: expected an operation, got {:?}", submitted.name);
            (Step::new(call, Verdict::ShapeChanged { field }), None)
        }
        Ok(submitted) => (Step::new(call, Verdict::Ok), Some(submitted.name)),
    }
}

/// What one poll said: `None` while the work is still going, and otherwise
/// the verdict on the finished operation and the video to download.
pub fn operation_step(answer: Result<Operation, HttpError>) -> Option<(Step, Option<String>)> {
    let call = "GET operation";
    let operation = match answer {
        Err(error) => return Some((Step::new(call, judge::gemini(&error)), None)),
        Ok(operation) if !operation.done => return None,
        Ok(operation) => operation,
    };
    let verdict = if let Some(failure) = &operation.error {
        Verdict::Refused {
            said: format!("the shot failed ({}): {}", failure.code, failure.message),
        }
    } else if let Some(uri) = operation.video_uri() {
        return Some((Step::new(call, Verdict::Ok), Some(uri.to_owned())));
    } else if operation.response.is_some() {
        Verdict::Refused {
            said: String::from(
                "finished with no video in it — Veo drops a shot its filters catch; the recorded body says why",
            ),
        }
    } else {
        Verdict::ShapeChanged {
            field: String::from("response.generateVideoResponse.generatedSamples[0].video.uri"),
        }
    };
    Some((Step::new(call, verdict), None))
}

/// What the download was.
pub fn video_step(answer: Result<Vec<u8>, HttpError>) -> Step {
    let call = "GET the video";
    match answer {
        Err(error) => Step::new(call, judge::gemini(&error)),
        Ok(bytes) if judge::is_mp4(&bytes) => {
            Step::new(call, Verdict::Ok).noting(format!("{} bytes of MP4", bytes.len()))
        }
        Ok(bytes) => Step::new(
            call,
            Verdict::ShapeChanged {
                field: format!("body: expected an MP4, got {}", judge::looks_like(&bytes)),
            },
        ),
    }
}
