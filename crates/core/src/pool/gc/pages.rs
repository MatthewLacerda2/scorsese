//! Pictures a page names in its text, which no field of the document records.
//!
//! A page reaches an imported picture by a relative URL in its own markup —
//! `<img src="../assets/photo.png">`, the pattern `docs/pages.md` shows — so
//! the assets table says nothing about the use. Collecting such a picture
//! breaks the page silently, where every other dangling reference is refused
//! by validation. So a picture a page's text names is kept, and said to be.

use std::fs;
use std::path::Path;

use crate::asset::{AssetId, AssetKind};
use crate::project::{ASSETS_DIR, Project};

/// An asset `gc` keeps because a page's text names its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedByPage {
    /// The asset kept.
    pub asset: AssetId,
    /// The first page, in table order, whose text names it.
    pub page: AssetId,
}

/// Every asset under `assets/` whose path a page's text contains, in table
/// order.
///
/// A plain substring match, so `../assets/photo.png` and `assets/photo.png`
/// both count, and so does a mention in a comment: the cost of a false match is
/// a file kept, the cost of a missed one is a file deleted. A page whose file
/// cannot be read names nothing.
pub fn named_by_pages(project: &Project, project_root: &Path) -> Vec<NamedByPage> {
    let pages: Vec<(&AssetId, String)> = project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Html)
        .filter_map(|page| {
            let file = page.path.as_ref()?.resolve(project_root).ok()?;
            Some((&page.id, fs::read_to_string(file).ok()?))
        })
        .collect();

    project
        .assets
        .iter()
        .filter_map(|asset| {
            // Only imported media is looked for: generated output is named
            // by the brief that made it, not by hand in a page.
            let path = asset.path.as_ref()?.as_str();
            if !path.starts_with(&format!("{ASSETS_DIR}/")) {
                return None;
            }
            let (page, _) = pages.iter().find(|(_, text)| text.contains(path))?;
            Some(NamedByPage {
                asset: asset.id.clone(),
                page: (*page).clone(),
            })
        })
        .collect()
}
