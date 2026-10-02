//! Spending a confirmed quote: every charged brief reserved from the user's
//! credits and queued as a job, in one transaction — all of it, or none.

use scorsese_core::{AssetId, AssetKind, Project};
use scorsese_providers::quote::Quote;
use scorsese_providers::{image, speech, video};
use serde_json::Value;

use super::super::{Caller, database};
use crate::credits::CreditError;
use crate::credits::generations::{Line, Request, Shot, Still, start};
use crate::db;
use crate::generations::Payload;
use crate::jobs::{JobView, kinds, store as jobs};

/// A brief that is to be paid for, with everything its audit row records.
#[derive(Debug, Clone)]
pub(super) struct Charged {
    asset: String,
    brief: String,
    cents: u64,
    what: What,
}

impl Charged {
    /// The brief's hash.
    pub(super) fn brief(&self) -> &str {
        &self.brief
    }
}

/// What kind of generation, and its particulars.
#[derive(Debug, Clone)]
enum What {
    Shot {
        model: &'static str,
        resolution: &'static str,
        seconds: u32,
        aspect: &'static str,
        prompt: String,
    },
    Line {
        model: &'static str,
        voice: String,
        text: String,
        settings: Value,
    },
    Still {
        model: &'static str,
        resolution: &'static str,
        aspect: &'static str,
        references: usize,
        prompt: String,
    },
}

/// Every charged item of `quote`, gathered from the project laid out at
/// `root` — the same gathering the quote itself priced.
pub(super) fn charged(quote: &Quote, project: &Project, root: &std::path::Path) -> Vec<Charged> {
    let mut charged = Vec::new();
    for item in &quote.items {
        let Some(charge) = &item.charge else { continue };
        let Some(asset) = project.asset(&AssetId::new(item.subject.clone())) else {
            continue;
        };
        let what = match asset.kind {
            AssetKind::GeneratedVideo => match video::Brief::of(project, root, asset) {
                Ok(brief) => What::Shot {
                    model: brief.request.model.as_str(),
                    resolution: brief.request.resolution.as_str(),
                    seconds: brief.request.seconds.get(),
                    aspect: brief.request.aspect.as_str(),
                    prompt: brief.prompt,
                },
                Err(_) => continue,
            },
            AssetKind::GeneratedImage => match image::Brief::of(project, root, asset) {
                Ok(brief) => What::Still {
                    model: brief.request.model.as_str(),
                    resolution: brief.request.size().as_str(),
                    aspect: brief.request.aspect.as_str(),
                    references: brief.reference_images.len(),
                    prompt: brief.prompt,
                },
                Err(_) => continue,
            },
            AssetKind::GeneratedAudio => match speech::Brief::of(asset) {
                Ok(brief) => What::Line {
                    model: brief.request.model.as_str(),
                    settings: serde_json::to_value(&brief.request).unwrap_or(Value::Null),
                    voice: brief.voice_id,
                    text: brief.text,
                },
                Err(_) => continue,
            },
            _ => continue,
        };
        charged.push(Charged {
            asset: item.subject.clone(),
            brief: charge.brief.clone(),
            cents: charge.cents,
            what,
        });
    }
    charged
}

/// Reserve and queue every one of `charged` for project `project`, whose
/// document is `document` — or refuse all of them, spending nothing, when the
/// balance cannot cover the lot. Each queued job, by asset.
pub(super) async fn spend(
    caller: &Caller<'_>,
    project: i64,
    document: &Project,
    charged: &[Charged],
) -> Result<Vec<(String, JobView)>, String> {
    let snapshot = serde_json::to_value(document).map_err(database)?;
    let pool = &caller.toolbox.pool;
    let mut tx = db::scoped(pool, caller.user).await.map_err(database)?;
    let mut queued = Vec::with_capacity(charged.len());
    for one in charged {
        let payload = Payload {
            project,
            asset: one.asset.clone(),
            brief: one.brief.clone(),
            document: snapshot.clone(),
        };
        let payload = serde_json::to_value(&payload).map_err(database)?;
        let kind = match one.what {
            What::Shot { .. } => kinds::VEO_SHOT,
            What::Line { .. } => kinds::SPOKEN_LINE,
            What::Still { .. } => kinds::STILL_IMAGE,
        };
        let job = jobs::enqueue(&mut tx, kind, &payload)
            .await
            .map_err(database)?;
        let request = request(one, project, caller.call, job.id);
        start(&mut tx, &request)
            .await
            .map_err(|error| match error {
                CreditError::Insufficient { .. } => error.to_string(),
                other => database(other),
            })?;
        queued.push((one.asset.clone(), job));
    }
    tx.commit().await.map_err(database)?;
    for (_, job) in &queued {
        caller.toolbox.queue.announce(caller.user, job);
    }
    Ok(queued)
}

/// The audit row's request for one charged brief.
fn request(one: &Charged, project: i64, call: i64, job: i64) -> Request<'_> {
    match &one.what {
        What::Shot {
            model,
            resolution,
            seconds,
            aspect,
            prompt,
        } => Request::Shot(Shot {
            project: Some(project),
            tool_call: Some(call),
            job: Some(job),
            model,
            resolution,
            seconds: *seconds,
            aspect,
            prompt,
            brief_hash: &one.brief,
            estimated_cents: one.cents,
        }),
        What::Line {
            model,
            voice,
            text,
            settings,
        } => Request::Line(Line {
            project: Some(project),
            tool_call: Some(call),
            job: Some(job),
            model,
            voice,
            text,
            settings,
            estimated_cents: one.cents,
        }),
        What::Still {
            model,
            resolution,
            aspect,
            references,
            prompt,
        } => Request::Still(Still {
            project: Some(project),
            tool_call: Some(call),
            job: Some(job),
            model,
            resolution,
            aspect,
            references: *references,
            prompt,
            brief_hash: &one.brief,
            estimated_cents: one.cents,
        }),
    }
}
