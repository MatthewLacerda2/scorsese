//! A page from the library, brought into a project (#1144).
//!
//! Media is referred to where the library keeps it; a page is **copied**, into
//! the project's `pages/`, exactly as `scorsese import` copies one locally
//! (`scorsese_core::pool::import_asset`). A page is a document the project
//! edits — `page_write` rewrites it in place — so it has to be the project's
//! own: editing the library's copy would change every project that used it.

use std::path::Path;

use scorsese_core::pool::asset_id_for;
use scorsese_core::{Asset, AssetId, AssetKind, PAGES_DIR, Project, ProjectPath};

use crate::projects::ProjectFiles;

/// Bring `text`, the library page called `name`, into `project`: a new `html`
/// asset with its file kept at `pages/<id>.html` — or, when a page of the
/// project already holds exactly this text, that page. The id, and whether
/// it was added.
pub(super) fn bring_page(
    project: &mut Project,
    files: &mut ProjectFiles,
    name: &str,
    text: String,
) -> Result<(AssetId, bool), String> {
    if let Some(known) = holding(project, files, &text) {
        return Ok((known, false));
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("page");
    let mut id = asset_id_for(project, stem);
    // A file under pages/ that no asset owns yet is somebody's work: never
    // written over.
    let mut suffix = 1;
    while files.get(&file_of(&id)).is_some() {
        suffix += 1;
        id = asset_id_for(project, &format!("{stem}-{suffix}"));
    }
    let path = file_of(&id);
    files.insert(&path, text)?;
    project.assets.push(Asset::imported(
        id.clone(),
        AssetKind::Html,
        ProjectPath::new(path),
    ));
    Ok((id, true))
}

/// Where the page `id` keeps its file.
fn file_of(id: &AssetId) -> String {
    format!("{PAGES_DIR}/{id}.html")
}

/// The page of `project` whose file holds exactly `text`, if any.
fn holding(project: &Project, files: &ProjectFiles, text: &str) -> Option<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Html)
        .find(|asset| {
            asset
                .path
                .as_ref()
                .and_then(|path| files.get(path.as_str()))
                == Some(text)
        })
        .map(|asset| asset.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "<!doctype html><title>t</title><p>hello</p>";

    fn blank() -> (Project, ProjectFiles) {
        let project = Project::new("p", scorsese_core::Fps::default());
        (project, ProjectFiles::default())
    }

    #[test]
    fn a_page_is_copied_into_pages_under_its_own_name() {
        let (mut project, mut files) = blank();
        let (id, added) = bring_page(&mut project, &mut files, "Intro.html", PAGE.into()).unwrap();
        assert!(added);
        let path = project
            .asset(&id)
            .and_then(|asset| asset.path.clone())
            .unwrap();
        assert_eq!(path.as_str(), format!("pages/{id}.html"));
        assert_eq!(files.get(path.as_str()), Some(PAGE));
    }

    #[test]
    fn the_same_page_again_is_the_page_already_there() {
        let (mut project, mut files) = blank();
        let (first, _) = bring_page(&mut project, &mut files, "a.html", PAGE.into()).unwrap();
        let (again, added) = bring_page(&mut project, &mut files, "b.html", PAGE.into()).unwrap();
        assert_eq!((again, added), (first, false));
        assert_eq!(project.assets.len(), 1);
    }

    #[test]
    fn a_file_nobody_owns_is_never_written_over() {
        let (mut project, mut files) = blank();
        files.insert("pages/intro.html", "mine".into()).unwrap();
        let (id, _) = bring_page(&mut project, &mut files, "intro.html", PAGE.into()).unwrap();
        assert_ne!(id.as_str(), "intro");
        assert_eq!(files.get("pages/intro.html"), Some("mine"));
    }
}
