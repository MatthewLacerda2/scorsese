//! A Veo shot as a job: submit once, keep the ticket, poll, keep the video.

use std::sync::Arc;
use std::time::Instant;

use scorsese_core::AssetId;
use scorsese_providers::video::{Brief, Progress, Ticket, VideoProvider};
use scorsese_render::Tools;

use super::land::{Work, database, document};
use super::{Timing, Vendors};
use crate::credits::generations::{self, Answer, Generation, Paid};
use crate::jobs::{Context, Handler, Job, Outcome};
use crate::library::Kind;
use crate::storage::Storage;
use crate::tools::lay_out;

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
    let paid = match work.paid().await {
        Ok(Some(paid)) => paid,
        Ok(None) => return work.settled().await,
        Err(why) => return Outcome::Failed(why),
    };
    // Only before anything was sent: a job holding a ticket has already paid,
    // and polls for what it paid for.
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
    let brief = match gather(work).await {
        Ok(brief) => brief,
        Err(why) => return work.refused(paid, why).await,
    };
    let provider: Provider = match vendors.video() {
        Ok(provider) => Arc::from(provider),
        Err(why) => return work.refused(paid, why).await,
    };
    let ticket = match &work.job.ticket {
        Some(ticket) => Ticket(ticket.clone()),
        None => match submit(work, paid, &provider, brief).await {
            Ok(ticket) => ticket,
            Err(outcome) => return outcome,
        },
    };
    wait(work, paid, &provider, ticket, timing).await
}

/// The brief, gathered from the document the job carries, laid out with the
/// user's files — and refused if it is not the one that was quoted.
async fn gather(work: &Work) -> Result<Brief, String> {
    let project = document(&work.payload)?;
    let context = &work.context;
    let folder = lay_out(context.pool(), &work.storage, work.job.user, &project).await?;
    let (root, asset) = (
        folder.root().to_path_buf(),
        AssetId::new(work.payload.asset.clone()),
    );
    let brief = tokio::task::spawn_blocking(move || {
        let asset = project
            .asset(&asset)
            .ok_or_else(|| format!("the project has no asset `{asset}`"))?;
        Brief::of(&project, &root, asset).map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "gathering the brief crashed; that is a bug".to_owned())??;
    drop(folder);
    if brief.digest() != work.payload.brief {
        return Err("the brief is not the one that was quoted".into());
    }
    Ok(brief)
}

/// Hand the brief to Veo, and commit the ticket before anything else.
async fn submit(
    work: &Work,
    paid: Paid,
    provider: &Provider,
    brief: Brief,
) -> Result<Ticket, Outcome> {
    let sending = Arc::clone(provider);
    let submitted = tokio::task::spawn_blocking(move || sending.submit(&brief)).await;
    let ticket = match submitted {
        Ok(Ok(ticket)) => ticket,
        Ok(Err(error)) => return Err(work.refused(paid, error.message).await),
        Err(_) => return Err(work.refused(paid, "submitting crashed".into()).await),
    };
    // Now, before anything else can go wrong: this ticket is the only record
    // that money was spent. A failure to keep it is logged and the job goes
    // on polling with the one it holds.
    if let Err(error) = work.context.keep_ticket(&ticket.0).await {
        eprintln!(
            "scorsese-server: job {}: keeping a ticket: {error}",
            work.job.id
        );
    }
    if let Generation::Shot(id) = paid.generation {
        let kept = async {
            let mut tx = work.context.scoped().await?;
            generations::keep_ticket(&mut tx, id, &ticket.0).await?;
            tx.commit().await
        };
        if let Err(error) = kept.await {
            eprintln!("{}", database(error));
        }
    }
    Ok(ticket)
}

/// Poll until the shot is ready, refused, or past patience.
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
        let progress = tokio::task::spawn_blocking(move || asking.poll(&polled)).await;
        match progress {
            Ok(Ok(Progress::Failed(why))) => return work.refused(paid, why).await,
            Ok(Ok(Progress::Ready(ready))) => {
                let fetching = Arc::clone(provider);
                let fetched = tokio::task::spawn_blocking(move || fetching.fetch(&ready)).await;
                match fetched {
                    Ok(Ok(bytes)) => return work.keep(paid, bytes, Kind::Video).await,
                    Ok(Err(error)) => eprintln!("scorsese-server: fetching a shot: {error}"),
                    Err(_) => eprintln!("scorsese-server: fetching a shot crashed"),
                }
            }
            // A network blip is not a refusal: the shot is still Google's,
            // and asking again is free.
            Ok(Ok(Progress::Waiting)) => {}
            Ok(Err(error)) => eprintln!("scorsese-server: polling a shot: {error}"),
            Err(_) => eprintln!("scorsese-server: polling a shot crashed"),
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
