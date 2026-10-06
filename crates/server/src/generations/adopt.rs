//! Bringing a user's generations into one of their projects.
//!
//! The project is opened as it is **now** — not as it was when a generation
//! was asked for — laid out with the user's files and every generation they
//! have (`tools::lay_out`), and `scorsese_providers`' own `adopt` points each
//! prompted asset whose *current* brief has a file at it. So an asset whose
//! brief was edited while its job ran is left alone, and one whose brief was
//! generated in another of the user's projects is picked up for free. What
//! was newly pointed somewhere is measured, as every command that generates
//! measures what landed, its clips are shortened to what it came out as where
//! they now outlast it (`scorsese_core::placing::fit_to_sources`, #825), and
//! the document is saved with its revision check.

use scorsese_core::placing::{Shortened, fit_to_sources};
use scorsese_core::{AssetId, Project, ProjectPath, Reprobe, probe_assets};
use scorsese_providers::{image, speech, video};
use scorsese_render::{Ffprobe, Tools};
use sqlx::postgres::PgPool;

use crate::db::UserId;
use crate::projects::{self, ProjectError, ProjectFiles};
use crate::storage::Storage;
use crate::tools::lay_out;

/// What [`adopt`] brought in.
#[derive(Debug, Default)]
pub struct Adopted {
    /// The assets that now point somewhere new.
    pub moved: Vec<AssetId>,
    /// The clips of those that outlasted what came back, and were shortened
    /// to it — each one a gap the cut now has, for whoever drives the edit.
    pub shortened: Vec<Shortened>,
}

/// Adopt into `user`'s project `id` whatever their library holds for its
/// current briefs, and save it.
pub async fn adopt(
    pool: &PgPool,
    storage: &Storage,
    tools: &Tools,
    user: UserId,
    id: i64,
) -> Result<Adopted, String> {
    for _ in 0..3 {
        let stored = match projects::open(pool, user, id).await {
            Ok(stored) => stored,
            Err(ProjectError::NotFound) => return Ok(Adopted::default()),
            Err(error) => return Err(error.to_string()),
        };
        let folder = lay_out(
            pool,
            storage,
            user,
            &stored.document,
            &ProjectFiles::default(),
        )
        .await?;
        let (root, tools) = (folder.root().to_path_buf(), tools.clone());
        let mut document = stored.document.clone();
        let before = stored
            .document
            .to_json()
            .map_err(|error| error.to_string())?;
        let (document, adopted) = tokio::task::spawn_blocking(move || {
            let adopted = pointed(&mut document, &root, &tools);
            (document, adopted)
        })
        .await
        .map_err(|_| "adopting a generation crashed; that is a bug".to_owned())?;
        if document.to_json().ok().as_ref() == Some(&before) {
            return Ok(adopted);
        }
        match projects::save(pool, user, id, stored.summary.revision, &document).await {
            Ok(_) => return Ok(adopted),
            Err(ProjectError::Conflict { .. }) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("the project kept changing while a generation was brought in".into())
}

/// Adopt in the folder at `root`, measure what now points somewhere new, and
/// fit its clips to what it measured.
fn pointed(document: &mut Project, root: &std::path::Path, tools: &Tools) -> Adopted {
    let before: Vec<(AssetId, Option<ProjectPath>, Option<String>)> = document
        .assets
        .iter()
        .map(|asset| (asset.id.clone(), asset.path.clone(), asset.sha256.clone()))
        .collect();
    let mut adopted = video::adopt(document, root);
    adopted.extend(image::adopt(document, root));
    adopted.extend(speech::adopt(document, root));
    let mut moved = Vec::new();
    for asset in &mut document.assets {
        let was = before.iter().find(|(id, _, _)| *id == asset.id);
        let same = was.is_some_and(|(_, path, sha)| *path == asset.path && *sha == asset.sha256);
        if adopted.contains(&asset.id) && !same {
            // A measurement of the previous generation is not one of this.
            asset.media = None;
            moved.push(asset.id.clone());
        }
    }
    if moved.is_empty() {
        return Adopted::default();
    }
    probe_assets(document, root, &Ffprobe::new(tools.clone()), Reprobe::Skip);
    let shortened = fit_to_sources(document, &moved);
    Adopted { moved, shortened }
}
