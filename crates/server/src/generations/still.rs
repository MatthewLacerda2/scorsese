//! A Gemini still as a job (#461): one call, the picture on the same
//! connection.
//!
//! [`line`](super::line)'s shape — no ticket, nothing to poll, so a job
//! interrupted mid-call is simply run again and the brief-hash lookup at its
//! start keeps a still that did land from being paid for twice — with
//! [`shot`](super::shot)'s gathering, because a still's brief hashes the bytes
//! of the pictures it names, and those are the user's files laid out.

use std::sync::Arc;

use scorsese_core::AssetId;
use scorsese_providers::image::Brief;
use scorsese_render::Tools;

use super::Vendors;
use super::land::{Work, document};
use crate::credits::generations::Answer;
use crate::jobs::{Context, Handler, Job, Outcome};
use crate::library::Kind;
use crate::projects::ProjectFiles;
use crate::storage::Storage;
use crate::tools::lay_out;

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
    let brief = match gather(work).await {
        Ok(brief) => brief,
        Err(why) => return work.refused(paid, why).await,
    };
    let provider = match vendors.image() {
        Ok(provider) => provider,
        Err(why) => return work.refused(paid, why).await,
    };
    let drawn = tokio::task::spawn_blocking(move || provider.draw(&brief)).await;
    match drawn {
        Ok(Ok(bytes)) => work.keep(paid, bytes, Kind::Image).await,
        Ok(Err(error)) => work.refused(paid, error.message).await,
        Err(_) => work.refused(paid, "drawing crashed".into()).await,
    }
}

/// The still's brief, gathered from the document the job carries, laid out
/// with the user's files — and refused if it is not the one that was quoted.
async fn gather(work: &Work) -> Result<Brief, String> {
    let project = document(&work.payload)?;
    let folder = lay_out(
        work.context.pool(),
        &work.storage,
        work.job.user,
        &project,
        &ProjectFiles::default(),
    )
    .await?;
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
        return Err("the still is not the one that was quoted".into());
    }
    Ok(brief)
}
