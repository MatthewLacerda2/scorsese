//! A Gemini still ordered in a half-price batch, as a job (#947, the web side
//! of #894): order once, keep the batch job's name, ask until it answers.
//!
//! [`shot`](super::shot)'s shape rather than [`still`](super::still)'s,
//! because a batch is the one way of drawing a still that is in flight — for
//! up to a day. The batch job's name is the ticket: committed to the job's row
//! and the audit row the moment Google accepts, and a job that comes back
//! from a restart holding one **asks and never orders again** (`jobs`, *Veo:
//! never pay twice*). Asking is free.
//!
//! **One still per batch.** The job's payload is one brief, so each still is
//! its own batch of one; a batch's answers are keyed by the file each picture
//! lands in, and a batch of one has one answer, so it is taken whatever its
//! key. Grouping a project's stills into one batch can come later — the half
//! price is per request, not per batch, so nothing is lost meanwhile.
//!
//! A batch that stops without answers (failed, cancelled or expired) bills
//! nothing and is settled free, as a refused still is; one that outlasts
//! [`Timing::batch_patience`] goes `stuck` with its reservation held, since
//! Google may still draw and bill it.

use std::sync::Arc;
use std::time::Instant;

use scorsese_providers::image::{Batch, ImageProvider};
use scorsese_render::Tools;

use super::land::Work;
use super::still::gather;
use super::{Timing, Vendors};
use crate::credits::generations::{Answer, Paid};
use crate::jobs::{Context, Handler, Job, Outcome};
use crate::library::Kind;
use crate::storage::Storage;

/// The handler for [`crate::jobs::kinds::BATCH_STILL`].
pub fn handler(
    storage: Storage,
    tools: Tools,
    vendors: Arc<dyn Vendors>,
    timing: Timing,
) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools, vendors) = (storage.clone(), tools.clone(), Arc::clone(&vendors));
        async move {
            match Work::new(job, context, storage, tools) {
                Ok(work) => batched(&work, vendors.as_ref(), timing).await,
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

type Provider = Arc<dyn ImageProvider + Send + Sync>;

async fn batched(work: &Work, vendors: &dyn Vendors, timing: Timing) -> Outcome {
    let paid = match work.paid().await {
        Ok(Some(paid)) => paid,
        Ok(None) => return work.settled().await,
        Err(why) => return Outcome::Failed(why),
    };
    // Only before anything was ordered: a job holding a ticket has already
    // paid, and asks after what it paid for.
    if work.job.ticket.is_none()
        && let Ok(Some(item)) = work.made().await
    {
        if let Err(why) = work
            .settle(paid, &Answer::Failed("already in your library".into()))
            .await
        {
            return Outcome::Failed(why);
        }
        return work.landed(&item).await;
    }
    let provider: Provider = match vendors.image() {
        Ok(provider) => Arc::from(provider),
        Err(why) if work.job.ticket.is_none() => return work.refused(paid, why).await,
        // Ordered already, so not a refusal: the batch is Google's and will
        // be billed. Failing puts nothing back; the ticket stays in the row.
        Err(why) => return Outcome::Stuck(format!("{why}; the batch is kept in the job")),
    };
    let ticket = match &work.job.ticket {
        Some(ticket) => ticket.clone(),
        None => match order(work, paid, &provider).await {
            Ok(ticket) => ticket,
            Err(outcome) => return outcome,
        },
    };
    wait(work, paid, &provider, ticket, timing).await
}

/// Order the still as a batch of one, and commit the batch's name before
/// anything else.
async fn order(work: &Work, paid: Paid, provider: &Provider) -> Result<String, Outcome> {
    let brief = match gather(work).await {
        Ok(brief) => brief,
        Err(why) => return Err(work.refused(paid, why).await),
    };
    let ordering = Arc::clone(provider);
    let ordered = tokio::task::spawn_blocking(move || ordering.order(&[&brief])).await;
    let ticket = match ordered {
        Ok(Ok(ticket)) => ticket,
        Ok(Err(error)) => return Err(work.refused(paid, error.message).await),
        Err(_) => return Err(work.refused(paid, "ordering crashed".into()).await),
    };
    work.keep_ticket(paid, &ticket).await;
    Ok(ticket)
}

/// Ask until the batch has answered, stopped, or outlasted patience.
async fn wait(
    work: &Work,
    paid: Paid,
    provider: &Provider,
    ticket: String,
    timing: Timing,
) -> Outcome {
    let started = Instant::now();
    loop {
        let (asking, operation) = (Arc::clone(provider), ticket.clone());
        let answer = tokio::task::spawn_blocking(move || asking.ask(&operation)).await;
        match answer {
            Ok(Ok(Batch::Stopped(why))) => return work.refused(paid, why).await,
            Ok(Ok(Batch::Finished(answers))) => {
                // A batch of one: its one answer is this still's.
                return match answers.into_iter().next() {
                    Some((_, Ok(bytes))) => work.keep(paid, bytes, Kind::Image).await,
                    Some((_, Err(why))) => work.refused(paid, why).await,
                    None => work.refused(paid, "the batch drew nothing".into()).await,
                };
            }
            // Still drawing, or a network blip: the batch is Google's either
            // way, and asking again is free.
            Ok(Ok(Batch::Running)) => {}
            Ok(Err(error)) => eprintln!("scorsese-server: asking after a batch: {error}"),
            Err(_) => eprintln!("scorsese-server: asking after a batch crashed"),
        }
        if started.elapsed() >= timing.batch_patience {
            return Outcome::Stuck(format!(
                "the batch had not answered after {} hours; its name is kept and so is what \
                 was reserved for it",
                timing.batch_patience.as_secs() / 3600
            ));
        }
        tokio::time::sleep(timing.batch_every).await;
    }
}
