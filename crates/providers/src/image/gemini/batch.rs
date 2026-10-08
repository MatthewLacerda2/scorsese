//! A batch of briefs as the Batch API's body, and its answer as a [`Batch`].
//!
//! The same translation as [`super::create`], into `generateContent`'s
//! spelling (`crate::api::gemini::batch`): the prompt, then each reference in
//! [`Brief::of`]'s order, the shape, the size, and a thinking level only when
//! the brief names one. Each request is keyed by the file its picture lands
//! in, so an answer finds its way home without the job's order mattering.
//!
//! `generateContent` takes no output type, so a batched picture may come back
//! a PNG where one drawn now is always a JPEG. It is kept under the brief's
//! `.jpg` name all the same — that name is the cache key — and that is safe
//! because ffmpeg, which decodes every still here, reads the bytes rather than
//! the extension (a PNG named `.jpg` probes as `png`, checked 2026-10-08).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::api::gemini::batch::request::{
    Batch as Job, Content, Create, GenerationConfig, ImageConfig, InlineData, InputConfig, Keyed,
    Metadata, Part, Request, Requests, ThinkingConfig,
};
use crate::api::gemini::batch::response::{Operation, State};

use super::super::{Batch, Brief};
use super::size_of;

/// The body that orders `briefs` as one job.
pub(super) fn create(briefs: &[&Brief]) -> Create {
    Create {
        batch: Job {
            display_name: format!("scorsese: {} stills", briefs.len()),
            input_config: InputConfig {
                requests: Requests {
                    requests: briefs.iter().map(|brief| keyed(brief)).collect(),
                },
            },
        },
    }
}

/// One brief, keyed.
fn keyed(brief: &Brief) -> Keyed {
    let mut parts = vec![Part::Text {
        text: brief.prompt.clone(),
    }];
    parts.extend(brief.reference_images.iter().map(|still| Part::Image {
        inline_data: InlineData {
            mime_type: still.mime_type.clone(),
            data: STANDARD.encode(&still.bytes),
        },
    }));
    Keyed {
        request: Request {
            contents: vec![Content { parts }],
            generation_config: GenerationConfig {
                response_modalities: vec!["IMAGE"],
                image_config: ImageConfig {
                    aspect_ratio: brief.request.aspect.as_str(),
                    image_size: size_of(brief.request.size()),
                },
                thinking_config: brief.request.thinking.map(|level| ThinkingConfig {
                    thinking_level: level.as_str(),
                }),
            },
        },
        metadata: Metadata { key: brief.key() },
    }
}

/// The job's answer, decoded.
pub(super) fn read(job: &Operation) -> Batch {
    match job.state() {
        State::Running => Batch::Running,
        State::Stopped(why) => Batch::Stopped(why),
        State::Succeeded => Batch::Finished(
            job.answers()
                .iter()
                .map(|answer| {
                    let picture = answer.picture().and_then(|data| {
                        STANDARD
                            .decode(data)
                            .map_err(|_| String::from("the picture was not valid base64"))
                    });
                    (answer.key().to_owned(), picture)
                })
                .collect(),
        ),
    }
}
