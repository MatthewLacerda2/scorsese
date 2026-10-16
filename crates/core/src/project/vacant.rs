//! The one check before a project is laid out: that nothing is in the way.
//!
//! Here, under [`Project::create`](super::Project::create), so that `scorsese
//! new` and `project_new` refuse the same directories (#1288) — it used to
//! live in the MCP server alone, and the command line laid a project over
//! whatever folder it was pointed at.

use std::path::Path;

use super::{PROJECT_FILE_NAME, SaveError};

/// Refuses a directory with anything already in it.
///
/// A project laid over what was already there is worse than an error: half the
/// directory is someone else's and there is nothing to say which half. The
/// ordinary case — a path that does not exist yet — reads as nothing in the
/// way, and so does a directory that exists and is empty.
pub(super) fn vacant(dir: &Path) -> Result<(), SaveError> {
    let path = dir.to_path_buf();
    if dir.join(PROJECT_FILE_NAME).exists() {
        return Err(SaveError::AlreadyAProject { path });
    }
    let Ok(mut entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    match entries.next() {
        None => Ok(()),
        Some(_) => Err(SaveError::NotEmpty { path }),
    }
}
