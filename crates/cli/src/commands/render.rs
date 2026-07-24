//! `scorsese render`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};
use scorsese_render::{Bitrate, FrameRange, RenderSettings, Renderer, Resolution, Tools};

pub fn run(
    project_dir: &Path,
    out: &Path,
    resolution: Resolution,
    fps: Option<Fps>,
    bitrate: Option<Bitrate>,
    range: Option<FrameRange>,
) -> Result<()> {
    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    // The project's own grid is the right default: rendering at the rate the
    // edit was authored against is the one output rate that needs no conform.
    let fps = fps.unwrap_or(project.timeline_fps);
    let settings = RenderSettings::new(resolution, fps).with_bitrate(bitrate);
    let range = range.unwrap_or(FrameRange::ALL);

    let tools = Tools::discover()?;
    let report = Renderer::new(&tools, settings)
        .render(&project, project_dir, range, out)
        .with_context(|| format!("rendering to {}", out.display()))?;

    println!(
        "Wrote {} — {} frames at {fps} fps, {} ({:.2}s)",
        out.display(),
        report.frames,
        report.resolution,
        report.seconds()
    );
    if let Some(bitrate) = bitrate {
        println!("  bitrate {bitrate}");
    }
    for note in &report.notes {
        println!("  note: {note}");
    }
    Ok(())
}
