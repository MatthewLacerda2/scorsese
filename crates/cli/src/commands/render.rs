//! `scorsese render`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};
use scorsese_render::{
    Bitrate, FrameRange, RenderSettings, Renderer, Resolution, SampleRate, Tools,
};

/// Everything the command line can say about the file to produce. Gathered into
/// one type rather than passed as seven positional arguments, where two
/// `Option<Bitrate>` next to each other are a bug waiting to be written.
pub struct Options {
    pub resolution: Resolution,
    pub fps: Option<Fps>,
    pub bitrate: Option<Bitrate>,
    pub sample_rate: SampleRate,
    pub audio_bitrate: Option<Bitrate>,
    pub range: Option<FrameRange>,
}

pub fn run(project_dir: &Path, out: &Path, options: Options) -> Result<()> {
    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    // The project's own grid is the right default: rendering at the rate the
    // edit was authored against is the one output rate that needs no conform.
    let fps = options.fps.unwrap_or(project.timeline_fps);
    let settings = RenderSettings::new(options.resolution, fps)
        .with_bitrate(options.bitrate)
        .with_audio(options.sample_rate, options.audio_bitrate);
    let range = options.range.unwrap_or(FrameRange::ALL);

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
    match report.seconds_of_audio {
        Some(seconds) => println!("  audio {} ({seconds:.2}s)", settings.sample_rate),
        None => println!("  silent — the project has no audio clips"),
    }
    if let Some(bitrate) = options.bitrate {
        println!("  bitrate {bitrate}");
    }
    for note in &report.notes {
        println!("  note: {note}");
    }
    Ok(())
}
