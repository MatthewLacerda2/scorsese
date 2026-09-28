//! An ElevenLabs line as a job: one call, the audio on the same connection.
//!
//! No ticket and no polling — speech is never in flight
//! (`scorsese_providers::speech`) — so a job interrupted mid-call is simply
//! run again, and the brief-hash lookup at its start is what keeps a line
//! that did land from being paid for twice.

use std::sync::Arc;

use scorsese_core::AssetId;
use scorsese_providers::speech::Brief;
use scorsese_render::Tools;

use super::Vendors;
use super::land::{Work, document};
use crate::credits::generations::Answer;
use crate::jobs::{Context, Handler, Job, Outcome};
use crate::library::Kind;
use crate::storage::Storage;

/// The handler for [`crate::jobs::kinds::SPOKEN_LINE`].
pub fn handler(storage: Storage, tools: Tools, vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools, vendors) = (storage.clone(), tools.clone(), Arc::clone(&vendors));
        async move {
            match Work::new(job, context, storage, tools) {
                Ok(work) => line(&work, vendors.as_ref()).await,
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

async fn line(work: &Work, vendors: &dyn Vendors) -> Outcome {
    let paid = match work.paid().await {
        Ok(Some(paid)) => paid,
        Ok(None) => return work.settled().await,
        Err(why) => return Outcome::Failed(why),
    };
    if let Ok(Some(item)) = work.made().await {
        if let Err(why) = work
            .settle(paid, &Answer::Failed("already in your library".into()))
            .await
        {
            return Outcome::Failed(why);
        }
        return work.landed(&item).await;
    }
    let brief = match gather(work) {
        Ok(brief) => brief,
        Err(why) => return work.refused(paid, why).await,
    };
    let provider = match vendors.speech() {
        Ok(provider) => provider,
        Err(why) => return work.refused(paid, why).await,
    };
    let spoken = tokio::task::spawn_blocking(move || provider.speak(&brief)).await;
    match spoken {
        Ok(Ok(bytes)) => work.keep(paid, bytes, Kind::Audio).await,
        Ok(Err(error)) => work.refused(paid, error.message).await,
        Err(_) => work.refused(paid, "speaking crashed".into()).await,
    }
}

/// The line's brief, from the document the job carries — refused if it is
/// not the one that was quoted.
fn gather(work: &Work) -> Result<Brief, String> {
    let project = document(&work.payload)?;
    let id = AssetId::new(work.payload.asset.clone());
    let asset = project
        .asset(&id)
        .ok_or_else(|| format!("the project has no asset `{id}`"))?;
    let brief = Brief::of(asset).map_err(|error| error.to_string())?;
    if brief.digest() != work.payload.brief {
        return Err("the line is not the one that was quoted".into());
    }
    Ok(brief)
}
