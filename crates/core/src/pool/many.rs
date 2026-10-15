//! Importing several paths in one pass, the way the CLI's `import` and MCP's
//! `import` both take them.
//!
//! **One that fails does not abort the ones behind it**: media arrives in sets,
//! and losing the nine files that worked to the one that did not would make
//! a set cost one call per file. What landed and what failed come back side
//! by side, with whether the document changed, so a surface saves once —
//! after the loop, never per path — and words the rest its own way.

use std::path::{Path, PathBuf};

use crate::probe::ProbeMedia;
use crate::project::Project;
use crate::{AssetKind, Import, ImportError, SequenceImport, import_path, import_sequence};

/// What importing several paths did: a report per path that landed, in the
/// order given, and the paths that did not, each with why.
#[derive(Debug)]
pub struct ImportBatch<R> {
    /// One per path that landed.
    pub reports: Vec<R>,
    /// One per path that failed, with why.
    pub failures: Vec<(PathBuf, ImportError)>,
    /// Whether anything was added to the document, so it needs saving. Media
    /// that was already in the pool adds nothing.
    pub needs_save: bool,
}

/// [`import_path`] for every path in `paths`, carrying on past a failure.
pub fn import_many(
    project: &mut Project,
    project_root: &Path,
    paths: &[impl AsRef<Path>],
    kind: Option<AssetKind>,
    probe: &dyn ProbeMedia,
) -> ImportBatch<Import> {
    let batch = each(paths, |path| {
        import_path(project, project_root, path, kind, probe)
    });
    let needs_save = batch
        .reports
        .iter()
        .flat_map(|report| &report.imported)
        .any(|one| !one.reused);
    ImportBatch {
        needs_save,
        ..batch
    }
}

/// [`import_sequence`] for every folder in `paths`, carrying on past a
/// failure.
pub fn import_sequences(
    project: &mut Project,
    project_root: &Path,
    paths: &[impl AsRef<Path>],
    probe: &dyn ProbeMedia,
) -> ImportBatch<SequenceImport> {
    let batch = each(paths, |path| {
        import_sequence(project, project_root, path, probe)
    });
    let needs_save = batch.reports.iter().any(|report| !report.existed);
    ImportBatch {
        needs_save,
        ..batch
    }
}

/// `import` over every path, sorted into what landed and what failed.
fn each<R>(
    paths: &[impl AsRef<Path>],
    mut import: impl FnMut(&Path) -> Result<R, ImportError>,
) -> ImportBatch<R> {
    let mut reports = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        let path = path.as_ref();
        match import(path) {
            Ok(report) => reports.push(report),
            Err(why) => failures.push((path.to_owned(), why)),
        }
    }
    ImportBatch {
        reports,
        failures,
        needs_save: false,
    }
}
