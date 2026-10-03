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
use crate::generations::Vendors;
use crate::jobs::{Context, Handler, Job, Outcome};
use crate::library::{Arrival, Item, Kind, Library};
use crate::storage::Storage;

/// The handler for [`crate::jobs::kinds::VOICE_DESIGN`].
pub fn design_handler(storage: Storage, tools: Tools, vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools, vendors) = (storage.clone(), tools.clone(), Arc::clone(&vendors));
        async move {
            let library = Library::new(
                context.pool().clone(),
                storage.clone(),
                tools,
                context.queue().clone(),
            );
            let work = Work {
                job,
                context,
                library,
                storage,
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
    storage: Storage,
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
    let designed = tokio::task::spawn_blocking(move || studio.candidates(&brief)).await;
    let came = match designed {
        Ok(Ok(came)) if !came.is_empty() => came,
        Ok(Ok(_)) => {
            return work
                .refused(paid, "the studio answered with no candidates".into())
                .await;
        }
        Ok(Err(error)) => return work.refused(paid, error.to_string()).await,
        Err(_) => return work.refused(paid, "designing crashed".into()).await,
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
        let mut tx = self.context.scoped().await.map_err(database)?;
        let paid = generations::for_job(&mut tx, self.job.id)
            .await
            .map_err(database)?;
        tx.commit().await.map_err(database)?;
        Ok(paid)
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
            let file = self.storage.scratch(self.job.user).with_extension("mp3");
            let written = file
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&file, &candidate.sample));
            if let Err(error) = written {
                eprintln!("scorsese-server: keeping a voice design sample: {error}");
                return Err("the server could not keep what came back".into());
            }
            let sample = sample_hash(brief, index + 1);
            let arrival = Arrival {
                file,
                name: format!("voice design sample {}.mp3", index + 1),
                kind: Kind::Audio,
                extension: "mp3".to_owned(),
                announced: None,
                brief_hash: None,
            };
            let item = self
                .library
                .keep_generated(self.job.user, &sample, arrival)
                .await
                .map_err(database)?;
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
        match self.settle(paid, &Answer::Failed(why.clone()), None).await {
            Ok(()) => Outcome::Failed(format!("{why} — nothing was charged")),
            Err(error) => Outcome::Failed(error),
        }
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
    let made =
        tokio::task::spawn_blocking(move || studio.keep(&brief, &chosen, &name, &passed_over))
            .await
            .map_err(|_| "keeping the voice crashed".to_owned())?
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
