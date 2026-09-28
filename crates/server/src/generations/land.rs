//! What both generation jobs do around the provider: find what pays for
//! them, keep what came back, settle, and bring it into the project.

use scorsese_core::Project;
use scorsese_render::Tools;
use serde_json::json;

use super::Payload;
use super::adopt::adopt;
use crate::credits::generations::{self, Answer, Paid};
use crate::jobs::{Context, Job, Outcome};
use crate::library::{Arrival, Item, Kind, Library};
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

    /// What pays for this job, or `None` once it has been settled.
    pub(super) async fn paid(&self) -> Result<Option<Paid>, String> {
        let mut tx = self.context.scoped().await.map_err(database)?;
        let paid = generations::for_job(&mut tx, self.job.id)
            .await
            .map_err(database)?;
        tx.commit().await.map_err(database)?;
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
        match self.settle(paid, &Answer::Failed(why.clone())).await {
            Ok(()) => Outcome::Failed(format!("{why} — nothing was charged")),
            Err(error) => Outcome::Failed(error),
        }
    }

    /// Keep `bytes` in the library under this brief, charge `paid`, and bring
    /// the file into the project.
    pub(super) async fn keep(&self, paid: Paid, bytes: Vec<u8>, kind: Kind) -> Outcome {
        let extension = if kind == Kind::Audio { "mp3" } else { "mp4" };
        let file = self
            .storage
            .scratch(self.job.user)
            .with_extension(extension);
        let written = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, &bytes));
        if let Err(error) = written {
            eprintln!("scorsese-server: keeping a generation: {error}");
            return Outcome::Failed("the server could not keep what came back".into());
        }
        let arrival = Arrival {
            file,
            name: format!("{}.{extension}", self.payload.asset),
            kind,
            extension: extension.to_owned(),
            announced: None,
            brief_hash: None,
        };
        let item = match self
            .library
            .keep_generated(self.job.user, &self.payload.brief, arrival)
            .await
        {
            Ok(item) => item,
            Err(error) => return Outcome::Failed(database(error)),
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
            Ok(moved) => Outcome::Done(json!({
                "item": item.id,
                "asset": self.payload.asset,
                "project": self.payload.project,
                "adopted": moved.iter().any(|id| id.as_str() == self.payload.asset),
            })),
            Err(error) => Outcome::Failed(format!(
                "generated and kept in your library as item {}, but the project could not be \
                 updated: {error}",
                item.id
            )),
        }
    }
}

/// The document a job carries, read.
pub(super) fn document(payload: &Payload) -> Result<Project, String> {
    Project::from_json(&payload.document.to_string())
        .map_err(|error| format!("the project the job carries does not load: {error}"))
}

/// A failure on the server, logged and said without its detail.
pub(super) fn database(error: impl std::fmt::Display) -> String {
    eprintln!("scorsese-server: generation job: {error}");
    "the server failed while keeping the books on this generation; the operator has the \
     detail"
        .to_owned()
}
