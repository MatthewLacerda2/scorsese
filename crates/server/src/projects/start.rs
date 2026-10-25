//! What a project is started for (#1016): a placement and a kind of video,
//! both optional, given to the assistant's `project_new` as the CLI and local
//! MCP take them. The web app asks for neither (#1531).
//!
//! The choice does two things, kept apart on purpose. **The brief** is
//! `scorsese_core::style::Start::brief`, written into the project's script
//! exactly as `scorsese new` and the local `project_new` write it — that is
//! what the assistant reads, and once written it is the assistant's to edit
//! like any other text. **The shape**: a platform frames the project in its
//! own aspect, which the document carries (#1278) and every render that names
//! no size is delivered at. **The columns** `platform` and `style` keep the
//! choice beside the project; they never enter the document (`docs/web.md`,
//! *The edit is a document*).

use scorsese_core::style::{SCRIPT_FILE, Start};
use scorsese_core::{Project, ProjectPath};
use sqlx::postgres::PgPool;

use super::{ProjectError, ProjectFiles, Stored, store};
use crate::db::UserId;

/// Store `project` as a new project of `user`'s, started for `start`: framed
/// in its platform's shape, with the brief in its script when anything was
/// chosen, and the choice recorded beside it. With nothing chosen it is a plain [`super::create`].
pub async fn begin(
    pool: &PgPool,
    user: UserId,
    mut project: Project,
    start: &Start,
) -> Result<Stored, ProjectError> {
    let mut kept = ProjectFiles::default();
    // Framed in the placement's shape (#1278): what the editor previews and
    // every render defaults to, read from the document from now on.
    if let Some(aspect) = start.aspect() {
        project.aspect = aspect;
    }
    if let Some(brief) = start.brief() {
        // A few kilobytes of fixed text, against a cap of a megabyte.
        kept.insert(SCRIPT_FILE, brief)
            .expect("a starting brief is a file a project keeps, well under the cap");
        project.script = Some(ProjectPath::new(SCRIPT_FILE));
    }
    let summary = store::insert(pool, user, &project, start, &kept).await?;
    Ok(Stored {
        summary,
        document: project,
    })
}
