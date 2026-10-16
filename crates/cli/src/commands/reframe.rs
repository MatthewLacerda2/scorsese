//! `scorsese reframe`

use std::path::Path;

use anyhow::Result;
use scorsese_core::Ratio;
use scorsese_render::reframe;

/// Changes the project's frame to `aspect`, saves it, and prints what moved
/// and what is left to settle.
///
/// The work is `scorsese_render::reframe`'s, which the MCP `project_reframe`
/// tool and the web server call too, so the three say the same thing about the
/// same project. A project already that shape is not written at all.
pub(crate) fn run(project_dir: &Path, aspect: Ratio) -> Result<()> {
    let mut project = super::open(project_dir)?;
    let reframed = reframe(&mut project, project_dir, aspect);
    if reframed.refit.changed() {
        super::save(&project, project_dir)?;
    }
    println!("{reframed}");
    Ok(())
}
