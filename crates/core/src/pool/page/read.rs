//! Reading a page's document back, exactly as it is on disk.
//!
//! The other half of [`super::write_page`], and the one the CLI's `page read`
//! and MCP's `page_read` share: find the asset, check it is a page, check its
//! path is one a project may hold, read the file.

use std::path::Path;

use crate::asset::{AssetId, AssetKind};
use crate::path::ProjectPath;
use crate::project::Project;

/// Why a page could not be read. Each surface words the missing asset its own
/// way — the fix it names is its own command (#1115) — so that variant carries
/// no hint.
#[derive(Debug, thiserror::Error)]
pub enum PageReadError {
    /// Nothing answers to the id.
    #[error("there is no asset `{id}`")]
    NoAsset {
        /// The id asked for.
        id: AssetId,
    },
    /// An asset of another kind, or an html asset with no file.
    #[error("`{id}` is not a page")]
    NotAPage {
        /// The id asked for.
        id: AssetId,
    },
    /// The page's path is not one a project may hold — only reachable when a
    /// document was edited by hand to point somewhere it cannot.
    #[error("the page `{id}` is at {path}, which {problem}")]
    BadPath {
        /// The page.
        id: AssetId,
        /// Its path, as written.
        path: ProjectPath,
        /// What is wrong with it.
        problem: String,
    },
    /// The file is where the document says, and could not be read.
    #[error("the page `{id}` at {path} could not be read: {source}")]
    Unreadable {
        /// The page.
        id: AssetId,
        /// Its path, as written.
        path: ProjectPath,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
}

/// The HTML of the page called `id`, as it is on disk under `project_root`.
pub fn read_page(
    project: &Project,
    project_root: &Path,
    id: &AssetId,
) -> Result<String, PageReadError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| PageReadError::NoAsset { id: id.clone() })?;
    let (AssetKind::Html, Some(path)) = (asset.kind, &asset.path) else {
        return Err(PageReadError::NotAPage { id: id.clone() });
    };
    path.check().map_err(|problem| PageReadError::BadPath {
        id: id.clone(),
        path: path.clone(),
        problem: problem.to_string(),
    })?;
    std::fs::read_to_string(path.resolve(project_root)).map_err(|source| {
        PageReadError::Unreadable {
            id: id.clone(),
            path: path.clone(),
            source,
        }
    })
}
