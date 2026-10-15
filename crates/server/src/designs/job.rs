//! The two jobs: a design, paid for, and keeping a candidate, free.
//!
//! A design job's life is a generation's (`crate::generations`): find the
//! open reservation that pays for it — none means an earlier run settled it,
//! so answer with what that run kept — then, if the user already has this
//! design, release it, since nothing needs sending; otherwise ask the studio,
//! keep the three samples in the library, and record the candidates and the
//! charge in one transaction. A studio that refuses settles it free.

use std::sync::Arc;

use scorsese_providers::voices::design::{Brief, Candidate};
use scorsese_render::Tools;
use serde_json::{Value, json};

use super::store::{self, Design, Sample};
use super::{DesignPayload, KeepPayload, sample_hash, samples};
use crate::credits::designs::candidates;
use crate::credits::generations::{self, Answer, Generation, Paid};
use crate::generations::{Vendors, paying, refusal};
use crate::jobs::{Context, Handler, Job, Outcome, blocking};
use crate::library::{Item, Kind, Library, Made, Unkept};
use crate::storage::Storage;

/// The handler for [`crate::jobs::kinds::VOICE_DESIGN`].
pub fn design_handler(storage: Storage, tools: Tools, vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools, vendors) = (storage.clone(), tools.clone(), Arc::clone(&vendors));
        async move {
            let library = Library::new(
                context.pool().clone(),
                storage,
                tools,
                context.queue().clone(),
            );
            let work = Work {
                job,
                context,
                library,
            };
            match serde_json::from_value(work.job.payload.clone()) {
                Ok(payload) => design(&work, &payload, vendors.as_ref()).await,
                Err(error) => {
                    Outcome::Failed(format!("the job does not describe a design: {error}"))
                }
            }
        }
    }
}

/// What a design job works with.
struct Work {
    job: Job,
    context: Context,
    library: Library,
}

async fn design(work: &Work, payload: &DesignPayload, vendors: &dyn Vendors) -> Outcome {
    let paid = match work.paid().await {
        Ok(Some(paid)) => paid,
        Ok(None) => {
            return match work.found(&payload.brief).await {
                Ok(Some(done)) => done,
                Ok(None) => Outcome::Failed("nothing is paying for this job any more".into()),
                Err(why) => Outcome::Failed(why),
            };
        }
        Err(why) => return Outcome::Failed(why),
    };
    if let Ok(Some(done)) = work.found(&payload.brief).await {
        let free = Answer::Failed("already designed — the samples are in your library".into());
        return match work.settle(paid, &free, None).await {
            Ok(()) => done,
            Err(why) => Outcome::Failed(why),
        };
    }
    let brief = match Brief::new(
        &payload.prompt,
        &payload.passage,
        payload.seed,
        payload.guidance,
    ) {
        Ok(brief) if brief.digest() == payload.brief => brief,
        Ok(_) => {
            return work
                .refused(paid, "the design is not the one that was quoted".into())
                .await;
        }
        Err(error) => return work.refused(paid, error.to_string()).await,
    };
    let studio = match vendors.studio() {
        Ok(studio) => studio,
        Err(why) => return work.refused(paid, why).await,
    };
    let designed = blocking("designing crashed", move || studio.candidates(&brief)).await;
    let came = match designed {
        Ok(Ok(came)) if !came.is_empty() => came,
        Ok(Ok(_)) => {
            return work
                .refused(paid, "the studio answered with no candidates".into())
                .await;
        }
        Ok(Err(error)) => return work.refused(paid, error.to_string()).await,
        Err(why) => return work.refused(paid, why).await,
    };
    let kept = match work.keep(&payload.brief, &came).await {
        Ok(kept) => kept,
        Err(why) => return Outcome::Failed(why),
    };
    let (recorded, items): (Vec<Sample>, Vec<Item>) = kept.into_iter().unzip();
    let first = items.first().map(|item| item.id);
    match work
        .settle(paid, &Answer::Worked(first), Some(&recorded))
        .await
    {
        Ok(()) => match work.found(&payload.brief).await {
            Ok(Some(done)) => done,
            Ok(None) => Outcome::Failed("the design was kept but cannot be read back".into()),
            Err(why) => Outcome::Failed(why),
        },
        Err(why) => Outcome::Failed(why),
    }
}

impl Work {
    /// What pays for this job, or `None` once it has been settled.
    async fn paid(&self) -> Result<Option<Paid>, String> {
        paying(&self.context, &self.job).await.map_err(database)
    }

    /// The user's design from this brief whose samples are all still in their
    /// library, as the job's result.
    async fn found(&self, brief: &str) -> Result<Option<Outcome>, String> {
        let mut tx = self.context.scoped().await.map_err(database)?;
        let design = store::find(&mut tx, brief).await.map_err(database)?;
        tx.commit().await.map_err(database)?;
        let Some(design) = design else {
            return Ok(None);
        };
        let items = samples(&self.library, self.job.user, &design)
            .await
            .map_err(database)?;
        Ok(items.map(|items| Outcome::Done(result(&design, &items))))
    }

    /// Keep every candidate's sample in the library, each under its own hash.
    async fn keep(&self, brief: &str, came: &[Candidate]) -> Result<Vec<(Sample, Item)>, String> {
        let mut kept = Vec::with_capacity(came.len());
        for (index, candidate) in came.iter().enumerate() {
            let sample = sample_hash(brief, index + 1);
            let made = Made {
                bytes: candidate.sample.clone(),
                kind: Kind::Audio,
                name: format!("voice design sample {}", index + 1),
                words: None,
            };
            let item = match self
                .library
                .keep_generated_bytes(self.job.user, &sample, made)
                .await
            {
                Ok(item) => item,
                Err(Unkept::Unwritten(error)) => {
                    eprintln!("scorsese-server: keeping a voice design sample: {error}");
                    return Err("the server could not keep what came back".into());
                }
                Err(Unkept::Refused(error)) => return Err(database(error)),
            };
            let recorded = Sample {
                generated_voice_id: candidate.generated_voice_id.clone(),
                sample,
                seconds: candidate.seconds,
            };
            kept.push((recorded, item));
        }
        Ok(kept)
    }

    /// Settle `paid` with `answer`, recording `came` on the design first when
    /// it worked — one transaction, so a charge always says what it bought.
    async fn settle(
        &self,
        paid: Paid,
        answer: &Answer,
        came: Option<&[Sample]>,
    ) -> Result<(), String> {
        let mut tx = self.context.scoped().await.map_err(database)?;
        if let (Some(came), Generation::Design(id)) = (came, paid.generation) {
            let came = serde_json::to_value(came).map_err(database)?;
            candidates(&mut tx, id, &came).await.map_err(database)?;
        }
        generations::finish(&mut tx, paid, answer)
            .await
            .map_err(database)?;
        tx.commit().await.map_err(database)
    }

    /// The studio's refusal: settled free, and said.
    async fn refused(&self, paid: Paid, why: String) -> Outcome {
        let settled = self.settle(paid, &Answer::Failed(why.clone()), None).await;
        refusal(why, settled)
    }
}

/// A design as a job's result: each candidate and the library item to play.
fn result(design: &Design, items: &[Item]) -> Value {
    let candidates: Vec<Value> = design
        .candidates
        .iter()
        .zip(items)
        .map(|(sample, item)| {
            json!({ "generated_voice_id": sample.generated_voice_id, "item": item.id })
        })
        .collect();
    json!({ "design": design.id, "candidates": candidates })
}

/// The handler for [`crate::jobs::kinds::VOICE_KEEP`]: a candidate made a
/// voice, and recorded. Spends nothing.
pub fn keep_handler(vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let vendors = Arc::clone(&vendors);
        async move {
            match serde_json::from_value(job.payload.clone()) {
                Ok(payload) => keep(&context, &payload, vendors.as_ref()).await,
                Err(error) => {
                    Outcome::Failed(format!("the job does not describe a voice: {error}"))
                }
            }
        }
    }
}

async fn keep(context: &Context, payload: &KeepPayload, vendors: &dyn Vendors) -> Outcome {
    match kept(context, payload, vendors).await {
        Ok(done) => Outcome::Done(done),
        Err(why) => Outcome::Failed(why),
    }
}

async fn kept(
    context: &Context,
    payload: &KeepPayload,
    vendors: &dyn Vendors,
) -> Result<Value, String> {
    let mut tx = context.scoped().await.map_err(database)?;
    let design = store::get(&mut tx, payload.design)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    let design = design.ok_or("that design is not one of yours")?;
    let studio = vendors.studio()?;
    let provider = studio.name();
    let (brief, passed_over) = (design.brief(), design.passed_over(&payload.chosen));
    let (chosen, name) = (payload.chosen.clone(), payload.name.clone());
    let made = blocking("keeping the voice crashed", move || {
        studio.keep(&brief, &chosen, &name, &passed_over)
    })
    .await?
    .map_err(|error| format!("{error} — nothing was created"))?;
    let mut tx = context.scoped().await.map_err(database)?;
    let voice = (made.id.as_str(), made.name.as_str());
    store::keep(&mut tx, design.id, &payload.chosen, voice, provider)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(json!({ "voice_id": made.id, "name": made.name, "design": design.id }))
}

/// A failure on the server, logged and said without its detail.
fn database(error: impl std::fmt::Display) -> String {
    eprintln!("scorsese-server: voice design job: {error}");
    "the server failed while keeping the books on this voice design; the operator has the \
     detail"
        .to_owned()
}
