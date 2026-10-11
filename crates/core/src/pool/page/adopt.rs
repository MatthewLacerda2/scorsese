//! Registering a page that is already in `pages/`, rather than copying it.
//!
//! A page too large for one argument, or built by code, reaches `pages/`
//! before anybody registers it (#1054). Copying it then would fork it: the
//! generator keeps writing `pages/<name>`, the asset reads `pages/<name>-2`,
//! and every later rebuild silently misses the render. So the file is
//! adopted — the asset points at it and nothing is copied — when no asset owns
//! it already. A file another page owns is still suffixed past: two pages are
//! two documents about to diverge, and writing over one would lose work.

use std::path::Path;

use crate::project::{PAGES_DIR, Project};

use super::super::naming::sanitise;

/// The name `source` already has in this project's `pages/`, when it may be
/// registered where it is: directly in `pages/`, under a name import would
/// have given it anyway, and the path of no asset.
pub(super) fn in_pages(project: &Project, project_root: &Path, source: &Path) -> Option<String> {
    let pages = project_root.join(PAGES_DIR).canonicalize().ok()?;
    let source = source.canonicalize().ok()?;
    if source.parent()? != pages {
        return None;
    }
    let name = source.file_name()?.to_str()?;
    (sanitise(name) == name && !owned(project, name)).then(|| name.to_owned())
}

/// Whether `pages/<name>` is holding `html` already, byte for byte, with no
/// asset pointing at it: the page being written, put there first by hand.
pub(super) fn orphan_with(project: &Project, project_root: &Path, name: &str, html: &str) -> bool {
    !owned(project, name)
        && std::fs::read(project_root.join(PAGES_DIR).join(name))
            .is_ok_and(|bytes| bytes == html.as_bytes())
}

/// Whether some asset's path is `pages/<name>`.
fn owned(project: &Project, name: &str) -> bool {
    let path = format!("{PAGES_DIR}/{name}");
    project
        .assets
        .iter()
        .any(|asset| asset.path.as_ref().is_some_and(|p| p.as_str() == path))
}
