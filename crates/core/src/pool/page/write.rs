//! Writing a page's document: creating the `html` asset and its file when the
//! name is new, replacing the file's text when it is a page already.
//!
//! **One call, both cases**, because an agent editing a page and an agent
//! starting one are doing the same thing — handing over the whole document —
//! and a separate "new" step would be one more name on a tool list every
//! model call pays for (#780). The one case refused is a name that is already
//! some other kind of asset: suffixing it out of the way would make a second
//! asset nobody asked for, and writing over it is not possible.
//!
//! Like import, nothing is hashed: a page is authored, and the point of
//! authoring is editing.

use std::path::{Path, PathBuf};

use crate::asset::{Asset, AssetId, AssetKind};
use crate::path::ProjectPath;
use crate::project::{PAGES_DIR, Project};

use super::super::naming::{asset_id_for, unique_file_name};

/// What [`write_page`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageWritten {
    /// The `html` asset the page is, to reference from a clip.
    pub id: AssetId,
    /// Where its document is, relative to the project root.
    pub path: ProjectPath,
    /// Whether this call made the asset, rather than rewriting one.
    pub created: bool,
}

/// Why a page could not be written. Nothing is changed by a refusal: the
/// project document is only touched once the file has landed.
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    /// No document was given. An empty page would capture as nothing at all,
    /// which is never what was meant.
    #[error("a page needs a document; this one is empty")]
    Empty,
    /// The name is an asset of another kind already.
    #[error("`{id}` is already {} asset, not a page; choose another name", named(*kind))]
    NotAPage {
        /// The asset the name answers to.
        id: AssetId,
        /// What it is.
        kind: AssetKind,
    },
    /// The page's path is not one a project may hold — only reachable when a
    /// document was edited by hand to point somewhere it cannot.
    #[error("the page `{id}` is at {path}, which {problem}")]
    BadPath {
        /// The page.
        id: AssetId,
        /// Its path, as written.
        path: String,
        /// What is wrong with it.
        problem: String,
    },
    /// `pages/` could not be made, or the file could not be written.
    #[error("cannot write {}: {source}", path.display())]
    Unwritable {
        /// The file or directory.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
}

/// Writes `html` as the page called `name`: the existing `html` asset of that
/// id, or — when nothing answers to it — a new one, its file under `pages/`.
///
/// The caller saves the project; until it does, a new page is a file with no
/// asset pointing at it, which is what collecting unused assets leaves behind
/// anyway. A rewrite changes no field of the document at all.
pub fn write_page(
    project: &mut Project,
    project_root: &Path,
    name: &str,
    html: &str,
) -> Result<PageWritten, PageError> {
    if html.trim().is_empty() {
        return Err(PageError::Empty);
    }
    if let Some(asset) = project.asset(&AssetId::new(name)) {
        return rewrite(asset, project_root, html);
    }
    let id = asset_id_for(project, name);
    let pages = project_root.join(PAGES_DIR);
    std::fs::create_dir_all(&pages).map_err(|source| PageError::Unwritable {
        path: pages.clone(),
        source,
    })?;
    let file_name = unique_file_name(&pages, Path::new(&format!("{id}.html")));
    let file = pages.join(&file_name);
    land(&file, html)?;
    let path = ProjectPath::new(format!("{PAGES_DIR}/{file_name}"));
    project
        .assets
        .push(Asset::imported(id.clone(), AssetKind::Html, path.clone()));
    Ok(PageWritten {
        id,
        path,
        created: true,
    })
}

/// Replaces the document of the page `asset` already is.
fn rewrite(asset: &Asset, project_root: &Path, html: &str) -> Result<PageWritten, PageError> {
    let id = asset.id.clone();
    let path = match (&asset.kind, &asset.path) {
        (AssetKind::Html, Some(path)) => path.clone(),
        (kind, _) => return Err(PageError::NotAPage { id, kind: *kind }),
    };
    path.check().map_err(|problem| PageError::BadPath {
        id: id.clone(),
        path: path.as_str().to_owned(),
        problem: problem.to_string(),
    })?;
    let file = path.resolve(project_root);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|source| PageError::Unwritable {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    land(&file, html)?;
    Ok(PageWritten {
        id,
        path,
        created: false,
    })
}

/// How a kind is spelled in the document, `an image` — the word a reader
/// would look for in `project.json`.
fn named(kind: AssetKind) -> String {
    let word = serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();
    let article = if word.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} `{word}`")
}

/// The bytes, all or nothing.
fn land(file: &Path, html: &str) -> Result<(), PageError> {
    crate::write::atomically(file, html).map_err(|source| PageError::Unwritable {
        path: file.to_path_buf(),
        source,
    })
}
