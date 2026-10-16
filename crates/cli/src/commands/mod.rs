//! One module per subcommand. Each is thin: it parses intent, calls the
//! library, and prints. Logic that the MCP server would also want lives in
//! `scorsese-core` or `scorsese-render`, never here.

pub(crate) mod assets;
pub(crate) mod caption;
pub(crate) mod check;
pub(crate) mod confirm;
pub(crate) mod describe;
pub(crate) mod design;
pub(crate) mod dissolve;
pub(crate) mod duck;
pub(crate) mod generate;
pub(crate) mod guide;
pub(crate) mod hear;
pub(crate) mod icons;
pub(crate) mod import;
pub(crate) mod level;
pub(crate) mod live;
pub(crate) mod look;
pub(crate) mod migrate;
pub(crate) mod new;
pub(crate) mod page;
pub(crate) mod prices;
pub(crate) mod probe;
pub(crate) mod reframe;
pub(crate) mod render;
pub(crate) mod sequence;
pub(crate) mod settings;
pub(crate) mod still;
pub(crate) mod stock;
pub(crate) mod synth;
pub(crate) mod vectorize;
pub(crate) mod voice;
pub(crate) mod voices;

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::Project;

/// Opens the project in `project_dir`, naming the folder when it cannot.
pub(crate) fn open(project_dir: &Path) -> Result<Project> {
    Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))
}

/// Writes `project` back to `project_dir`.
pub(crate) fn save(project: &Project, project_dir: &Path) -> Result<()> {
    project.save(project_dir).context("saving the project")
}
