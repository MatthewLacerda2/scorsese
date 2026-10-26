//! What every generation job does around the provider: find what pays for
//! them, keep what came back, settle, and bring it into the project.

use scorsese_core::Project;
use scorsese_core::placing::Shortened;
use scorsese_core::words::Words;
use scorsese_render::Tools;
use serde_json::json;

use super::Payload;
use super::adopt::adopt;
use crate::credits::generations::{self, Answer, Paid};
use crate::db::{self, Says};
use crate::jobs::{Context, Job, Outcome};
use crate::library::{Item, Kind, Library, Made, Unkept};
use crate::storage::Storage;

/// What a generation job works with.
pub(super) struct Work {
    pub(super) job: Job,
    pub(super) context: Context,
    pub(super) payload: Payload,
    pub(super) library: Library,
    pub(super) storage: Storage,
    pub(super) tools: Tools,
}

impl Work {
    /// Read `job`'s payload, and gather what it works with.
    pub(super) fn new(
        job: Job,
        context: Context,
        storage: Storage,
        tools: Tools,
    ) -> Result<Self, String> {
        let payload: Payload = serde_json::from_value(job.payload.clone())
            .map_err(|error| format!("the job does not describe a generation: {error}"))?;
        let library = Library::new(
            context.pool().clone(),
            storage.clone(),
            tools.clone(),
            context.queue().clone(),
        );
        Ok(Self {
            job,
            context,
            payload,
            library,
            storage,
            tools,
        })
    }

    /// Commit a provider's `ticket` to the job's row and to `paid`'s audit
    /// row — before anything else, since it is the only record that money
    /// was spent. A failure to keep it is logged, and the job goes on
    /// polling with the one it holds.
    pub(super) async fn keep_ticket(&self, paid: Paid, ticket: &str) {
        if let Err(error) = self.context.keep_ticket(ticket).await {
            tracing::error!("job {}: keeping a ticket: {error}", self.job.id);
        }
        let kept = async {
            let mut tx = self.context.scoped().await?;
            generations::keep_ticket(&mut tx, paid.generation, ticket).await?;
            tx.commit().await
        };
        if let Err(error) = kept.await {
            tracing::error!("{}", database(error));
        }
    }

    /// What pays for this job — or, when nothing is left to pay for, the
    /// job's answer: what the library holds once an earlier run settled it,
    /// or what it already held for this brief, released free.
    pub(super) async fn begin(&self) -> Result<Paid, Outcome> {
        let paid = match paying(&self.context, &self.job).await {
            Ok(Some(paid)) => paid,
            Ok(None) => return Err(self.settled().await),
            Err(error) => return Err(Outcome::Failed(database(error))),
        };
        // Only before anything was sent: a job holding a ticket has already
        // paid, and asks after what it paid for. A still or a line never
        // holds one.
        if self.job.ticket.is_none()
            && let Ok(Some(item)) = self.made().await
        {
            if let Err(why) = self
                .settle(paid, &Answer::Failed("already in your library".into()))
                .await
            {
                return Err(Outcome::Failed(why));
            }
            return Err(self.landed(&item).await);
        }
        Ok(paid)
    }

    /// The generation the library already holds for this brief.
    pub(super) async fn made(&self) -> Result<Option<Item>, String> {
        self.library
            .find_generated(self.job.user, &self.payload.brief)
            .await
            .map_err(database)
    }

    /// Settle `paid` with `answer`.
    pub(super) async fn settle(&self, paid: Paid, answer: &Answer) -> Result<(), String> {
        let mut tx = self.context.scoped().await.map_err(database)?;
        generations::finish(&mut tx, paid, answer)
            .await
            .map_err(database)?;
        tx.commit().await.map_err(database)
    }

    /// A provider's refusal: settled free, and said.
    pub(super) async fn refused(&self, paid: Paid, why: String) -> Outcome {
        let settled = self.settle(paid, &Answer::Failed(why.clone())).await;
        refusal(why, settled)
    }

    /// Keep `bytes` in the library under this brief, charge `paid`, and bring
    /// the file into the project.
    pub(super) async fn keep(&self, paid: Paid, bytes: Vec<u8>, kind: Kind) -> Outcome {
        self.keep_timed(paid, bytes, kind, None).await
    }

    /// [`Work::keep`], with a spoken line's word timings kept on its item
    /// (#886) — `None` for output that has none.
    pub(super) async fn keep_timed(
        &self,
        paid: Paid,
        bytes: Vec<u8>,
        kind: Kind,
        words: Option<Words>,
    ) -> Outcome {
        let made = Made {
            bytes,
            kind,
            name: self.payload.asset.clone(),
            words,
        };
        let kept = self
            .library
            .keep_generated_bytes(self.job.user, &self.payload.brief, made)
            .await;
        let item = match kept {
            Ok(item) => item,
            Err(Unkept::Unwritten(error)) => {
                tracing::error!("keeping a generation: {error}");
                return Outcome::Failed("the server could not keep what came back".into());
            }
            Err(Unkept::Refused(error)) => return Outcome::Failed(database(error)),
        };
        if let Err(error) = self.settle(paid, &Answer::Worked(Some(item.id))).await {
            return Outcome::Failed(error);
        }
        self.landed(&item).await
    }

    /// A job whose payment was already settled — run again after a restart
    /// that came after it finished: bring in what the library holds, if
    /// anything.
    pub(super) async fn settled(&self) -> Outcome {
        match self.made().await {
            Ok(Some(item)) => self.landed(&item).await,
            Ok(None) => Outcome::Failed("nothing is paying for this job any more".into()),
            Err(why) => Outcome::Failed(why),
        }
    }

    /// Bring what the library holds for this brief into the project, and say
    /// so.
    pub(super) async fn landed(&self, item: &Item) -> Outcome {
        let adopted = adopt(
            self.context.pool(),
            &self.storage,
            &self.tools,
            self.job.user,
            self.payload.project,
        )
        .await;
        match adopted {
            Ok(adopted) => Outcome::Done(json!({
                "item": item.id,
                "asset": self.payload.asset,
                "project": self.payload.project,
                "adopted": adopted.moved.iter().any(|id| id.as_str() == self.payload.asset),
                "shortened": adopted.shortened.iter().map(Shortened::says).collect::<Vec<_>>(),
            })),
            Err(error) => Outcome::Failed(format!(
                "generated and kept in your library as item {}, but the project could not be \
                 updated: {error}",
                item.id
            )),
        }
    }
}

/// What pays for `job`, or `None` once it has been settled — for a design job
/// too (`crate::designs`), which keeps its own books' wording.
pub(crate) async fn paying(context: &Context, job: &Job) -> Result<Option<Paid>, sqlx::Error> {
    let mut tx = context.scoped().await?;
    let paid = generations::for_job(&mut tx, job.id).await?;
    tx.commit().await?;
    Ok(paid)
}

/// A provider's refusal, once `settled` free: said, or the settling's own
/// failure.
pub(crate) fn refusal(why: String, settled: Result<(), String>) -> Outcome {
    match settled {
        Ok(()) => Outcome::Failed(format!("{why} — nothing was charged")),
        Err(error) => Outcome::Failed(error),
    }
}

/// The document a job carries, read.
pub(super) fn document(payload: &Payload) -> Result<Project, String> {
    Project::from_json(&payload.document.to_string())
        .map_err(|error| format!("the project the job carries does not load: {error}"))
}

/// A failure on the server, logged and said without its detail.
pub(super) fn database(error: impl std::fmt::Display) -> String {
    db::user_facing("generation job", Says::Books("generation"), error)
}
