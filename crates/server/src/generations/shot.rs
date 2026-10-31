//! A Veo shot as a job: submit once, keep the ticket, poll, keep the video.

use std::sync::Arc;
use std::time::Instant;

use scorsese_providers::video::{Brief, Progress, Ticket, VideoProvider};
use scorsese_render::Tools;

use super::land::Work;
use super::{Timing, Vendors};
use crate::credits::generations::Paid;
use crate::jobs::{Context, Handler, Job, Outcome, blocking};
use crate::library::Kind;
use crate::storage::Storage;

/// The handler for [`crate::jobs::kinds::VEO_SHOT`].
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
                Ok(work) => shot(&work, vendors.as_ref(), timing).await,
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

type Provider = Arc<dyn VideoProvider + Send + Sync>;

async fn shot(work: &Work, vendors: &dyn Vendors, timing: Timing) -> Outcome {
    let paid = match work.begin().await {
        Ok(paid) => paid,
        Err(done) => return done,
    };
    let provider: Provider = match vendors.video() {
        Ok(provider) => Arc::from(provider),
        Err(why) if work.job.ticket.is_none() => return work.refused(paid, why).await,
        // Submitted already, so not a refusal: the shot is Google's and will
        // be billed. The ticket stays in the row, and so does the reservation.
        Err(why) => return Outcome::Stuck(format!("{why}; the ticket is kept in the job")),
    };
    let ticket = match &work.job.ticket {
        // The brief is only ever sent once: a job holding a ticket asks after
        // it, and never gathers it again.
        Some(ticket) => Ticket(ticket.clone()),
        None => match submit(work, paid, &provider).await {
            Ok(ticket) => ticket,
            Err(outcome) => return outcome,
        },
    };
    wait(work, paid, &provider, ticket, timing).await
}

/// The brief, gathered from the document the job carries, laid out with the
/// user's files — and refused if it is not the one that was quoted.
async fn gather(work: &Work) -> Result<Brief, String> {
    let brief = work
        .laid_out(|project, root, asset| {
            Brief::of(project, root, asset).map_err(|error| error.to_string())
        })
        .await?;
    work.quoted(&brief.digest(), "the brief")?;
    Ok(brief)
}

/// Gather the brief and hand it to Veo, and commit the ticket before anything
/// else.
async fn submit(work: &Work, paid: Paid, provider: &Provider) -> Result<Ticket, Outcome> {
    let brief = match gather(work).await {
        Ok(brief) => brief,
        Err(why) => return Err(work.refused(paid, why).await),
    };
    let sending = Arc::clone(provider);
    let submitted = blocking("submitting crashed", move || sending.submit(&brief)).await;
    let ticket = match submitted {
        Ok(Ok(ticket)) => ticket,
        Ok(Err(error)) => return Err(work.refused(paid, error.message).await),
        Err(why) => return Err(work.refused(paid, why).await),
    };
    // Now, before anything else can go wrong: this ticket is the only record
    // that money was spent.
    work.keep_ticket(paid, &ticket.0).await;
    Ok(ticket)
}

/// Poll until the shot is ready, refused, lost, or past patience.
async fn wait(
    work: &Work,
    paid: Paid,
    provider: &Provider,
    ticket: Ticket,
    timing: Timing,
) -> Outcome {
    let started = Instant::now();
    loop {
        let (asking, polled) = (Arc::clone(provider), ticket.clone());
        let progress = blocking("polling a shot crashed", move || asking.poll(&polled)).await;
        match progress {
            Ok(Ok(Progress::Failed(why))) => return work.refused(paid, why).await,
            Ok(Ok(Progress::Ready(ready))) => {
                let fetching = Arc::clone(provider);
                let fetched =
                    blocking("fetching a shot crashed", move || fetching.fetch(&ready)).await;
                match fetched {
                    Ok(Ok(bytes)) => return work.keep(paid, bytes, Kind::Video).await,
                    Ok(Err(error)) => tracing::warn!("fetching a shot: {error}"),
                    Err(why) => tracing::warn!("{why}"),
                }
            }
            // A network blip is not a refusal: the shot is still Google's,
            // and asking again is free.
            Ok(Ok(Progress::Waiting)) => {}
            // Unless Veo says it will never answer for this ticket (#1196):
            // waiting out patience on it would only keep the reservation. It
            // settles as a refusal does — nothing charged, the share given
            // back — since the provider delivered nothing for it.
            Ok(Err(error)) if error.lost => {
                let why = format!("Veo no longer knows this shot's ticket: {}", error.message);
                return work.refused(paid, why).await;
            }
            Ok(Err(error)) => tracing::warn!("polling a shot: {error}"),
            Err(why) => tracing::warn!("{why}"),
        }
        if started.elapsed() >= timing.patience {
            return Outcome::Stuck(format!(
                "Veo was still generating after {} minutes; the ticket is kept and so is \
                 what was reserved for it",
                timing.patience.as_secs() / 60
            ));
        }
        tokio::time::sleep(timing.poll_every).await;
    }
}
