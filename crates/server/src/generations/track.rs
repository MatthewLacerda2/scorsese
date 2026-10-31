//! An ElevenLabs Music track as a job (#1264): [`line`](super::line)'s
//! shape — one call, the audio on the same connection, no ticket — so a job
//! interrupted mid-call is simply run again, and the brief-hash lookup at its
//! start keeps a track that did land from being paid for twice.

use std::sync::Arc;

use scorsese_providers::music::Brief;
use scorsese_render::Tools;

use super::Vendors;
use super::land::Work;
use crate::jobs::{Context, Handler, Job, Outcome, blocking};
use crate::library::Kind;
use crate::storage::Storage;

/// The handler for [`crate::jobs::kinds::MUSIC_TRACK`].
pub fn handler(storage: Storage, tools: Tools, vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools, vendors) = (storage.clone(), tools.clone(), Arc::clone(&vendors));
        async move {
            match Work::new(job, context, storage, tools) {
                Ok(work) => track(&work, vendors.as_ref()).await,
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

async fn track(work: &Work, vendors: &dyn Vendors) -> Outcome {
    let paid = match work.begin().await {
        Ok(paid) => paid,
        Err(done) => return done,
    };
    let brief = match gather(work) {
        Ok(brief) => brief,
        Err(why) => return work.refused(paid, why).await,
    };
    let provider = match vendors.music() {
        Ok(provider) => provider,
        Err(why) => return work.refused(paid, why).await,
    };
    match blocking("composing crashed", move || provider.compose(&brief)).await {
        Ok(Ok(bytes)) => work.keep(paid, bytes, Kind::Audio).await,
        Ok(Err(error)) => work.refused(paid, error.message).await,
        Err(why) => work.refused(paid, why).await,
    }
}

/// The track's brief, from the document the job carries — refused if it is
/// not the one that was quoted.
fn gather(work: &Work) -> Result<Brief, String> {
    let project = work.document()?;
    let id = work.asset();
    let asset = project
        .asset(&id)
        .ok_or_else(|| format!("the project has no asset `{id}`"))?;
    let brief = Brief::of(asset).map_err(|error| error.to_string())?;
    work.quoted(&brief.digest(), "the track")?;
    Ok(brief)
}
