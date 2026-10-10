//! What a project is started for (#1016): a placement and a kind of video,
//! both optional, chosen in the web app's new-project dialog.
//!
//! The choice does two things, kept apart on purpose. **The brief** is
//! `scorsese_core::style::Start::brief`, written into the project's script
//! exactly as `scorsese new` and the local `project_new` write it — that is
//! what the assistant reads, and once written it is the assistant's to edit
//! like any other text. **The columns** `platform` and `style` are the web's
//! memory of the choice: what the editor shows as chosen, and what a render
//! that names no size is delivered at. They never enter the document
//! (`docs/web.md`, *The edit is a document*), and changing them later rewrites
//! no script: the assistant is told, and the script is its to change.

use scorsese_core::style::{SCRIPT_FILE, Start, style};
use scorsese_core::{Project, ProjectPath};
use sqlx::postgres::PgPool;

use super::{ProjectError, ProjectFiles, Stored, store};
use crate::db::{self, UserId};

/// Store `project` as a new project of `user`'s, started for `start`: with
/// the brief in its script when anything was chosen, and the choice recorded
/// beside it. With nothing chosen it is a plain [`super::create`].
pub async fn begin(
    pool: &PgPool,
    user: UserId,
    mut project: Project,
    start: &Start,
) -> Result<Stored, ProjectError> {
    let mut kept = ProjectFiles::default();
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

/// Record that `user`'s project `id` is now made for `start`, answering what
/// it was made for before. The document and its script are left alone, and
/// the revision does not move: nothing in the edit changed.
pub async fn retarget(
    pool: &PgPool,
    user: UserId,
    id: i64,
    start: &Start,
) -> Result<Start, ProjectError> {
    let mut tx = db::scoped(pool, user).await?;
    let before: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT platform, style FROM projects WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let (platform, chosen) = before.ok_or(ProjectError::NotFound)?;
    sqlx::query("UPDATE projects SET platform = $2, style = $3 WHERE id = $1")
        .bind(id)
        .bind(start.platform.map(|platform| platform.id()))
        .bind(start.style.map(|style| style.id))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Start {
        platform: platform.and_then(|id| id.parse().ok()),
        style: chosen.as_deref().and_then(style),
    })
}
