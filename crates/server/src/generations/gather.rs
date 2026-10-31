//! A generation's brief, gathered again from the document its job carries —
//! and refused if it is not the one that was quoted.

use std::path::Path;

use scorsese_core::{Asset, Project};

use super::land::Work;
use crate::jobs::blocking;
use crate::projects::ProjectFiles;
use crate::tools::lay_out;

impl Work {
    /// The brief `of` makes from the job's asset, with the document laid out
    /// with the user's files — for a brief that hashes the bytes of the
    /// files it names (a shot, a still).
    pub(super) async fn laid_out<B, F>(&self, of: F) -> Result<B, String>
    where
        B: Send + 'static,
        F: FnOnce(&Project, &Path, &Asset) -> Result<B, String> + Send + 'static,
    {
        let project = self.document()?;
        let folder = lay_out(
            self.context.pool(),
            &self.storage,
            self.job.user,
            &project,
            &ProjectFiles::default(),
        )
        .await?;
        let (root, asset) = (folder.root().to_path_buf(), self.asset());
        let brief = blocking("gathering the brief crashed; that is a bug", move || {
            let asset = project
                .asset(&asset)
                .ok_or_else(|| format!("the project has no asset `{asset}`"))?;
            of(&project, &root, asset)
        })
        .await??;
        drop(folder);
        Ok(brief)
    }

    /// Refuse a brief whose `digest` is not the one the job was quoted for;
    /// `what` names it in the refusal ("the brief", "the still").
    pub(super) fn quoted(&self, digest: &str, what: &str) -> Result<(), String> {
        if digest == self.payload.brief {
            Ok(())
        } else {
            Err(format!("{what} is not the one that was quoted"))
        }
    }
}
