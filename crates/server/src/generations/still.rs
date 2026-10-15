//! A Gemini still as a job (#461): one call, the picture on the same
//! connection.
//!
//! [`line`](super::line)'s shape — no ticket, nothing to poll, so a job
//! interrupted mid-call is simply run again and the brief-hash lookup at its
//! start keeps a still that did land from being paid for twice — with
//! [`shot`](super::shot)'s gathering, because a still's brief hashes the bytes
//! of the pictures it names, and those are the user's files laid out.

use std::sync::Arc;

use scorsese_providers::image::Brief;
use scorsese_render::Tools;

use super::Vendors;
use super::land::Work;
use crate::jobs::{Context, Handler, Job, Outcome, blocking};
use crate::library::Kind;
use crate::storage::Storage;

/// The handler for [`crate::jobs::kinds::STILL_IMAGE`].
pub fn handler(storage: Storage, tools: Tools, vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools, vendors) = (storage.clone(), tools.clone(), Arc::clone(&vendors));
        async move {
            match Work::new(job, context, storage, tools) {
                Ok(work) => still(&work, vendors.as_ref()).await,
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

async fn still(work: &Work, vendors: &dyn Vendors) -> Outcome {
    let paid = match work.begin().await {
        Ok(paid) => paid,
        Err(done) => return done,
    };
    let brief = match gather(work).await {
        Ok(brief) => brief,
        Err(why) => return work.refused(paid, why).await,
    };
    let provider = match vendors.image() {
        Ok(provider) => provider,
        Err(why) => return work.refused(paid, why).await,
    };
    match blocking("drawing crashed", move || provider.draw(&brief)).await {
        Ok(Ok(bytes)) => work.keep(paid, bytes, Kind::Image).await,
        Ok(Err(error)) => work.refused(paid, error.message).await,
        Err(why) => work.refused(paid, why).await,
    }
}

/// The still's brief, gathered from the document the job carries, laid out
/// with the user's files — and refused if it is not the one that was quoted.
pub(super) async fn gather(work: &Work) -> Result<Brief, String> {
    let brief = work
        .laid_out(|project, root, asset| {
            Brief::of(project, root, asset).map_err(|error| error.to_string())
        })
        .await?;
    work.quoted(&brief.digest(), "the still")?;
    Ok(brief)
}
