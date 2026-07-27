//! `scorsese render`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};
use scorsese_render::{
    AudioCodec, Bitrate, Container, FrameRange, OutputFormat, RenderSettings, Renderer, Resolution,
    SampleRate, Tools, VideoCodec,
};

/// Everything the command line can say about the file to produce. Gathered into
/// one type rather than passed as seven positional arguments, where two
/// `Option<Bitrate>` next to each other are a bug waiting to be written.
pub struct Options {
    /// The canvas every layer is composited onto, and so the size of the file.
    pub resolution: Resolution,
    /// `None` means the project's own timeline rate — the one output rate that
    /// conforms nothing.
    pub fps: Option<Fps>,
    /// `None` leaves the video encoder aiming for constant quality instead.
    pub bitrate: Option<Bitrate>,
    /// The rate the mix is produced at; sources recorded at others meet it.
    pub sample_rate: SampleRate,
    /// `None` leaves the audio encoder on its own default.
    pub audio_bitrate: Option<Bitrate>,
    /// `None` renders the whole timeline.
    pub range: Option<FrameRange>,
    /// `None` takes the container `out`'s extension asks for.
    pub container: Option<Container>,
    /// `None` takes the picture codec the container is written with.
    pub video_codec: Option<VideoCodec>,
    /// `None` takes the sound codec the container is written with.
    pub audio_codec: Option<AudioCodec>,
}

/// Renders the project to `out`, then prints what was written — for a headless
/// render those lines are the only report anyone gets.
pub fn run(project_dir: &Path, out: &Path, options: Options) -> Result<()> {
    // First, before the project is even opened: what shape the file is asked
    // to be is the cheapest thing to get wrong and the most expensive thing to
    // find out late. A combination we do not write is refused here, with
    // nothing spent and no ffmpeg yet located.
    let container = match options.container {
        Some(container) => container,
        None => Container::from_path(out)?,
    };
    let format = OutputFormat::new(container, options.video_codec, options.audio_codec)?;

    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    // The project's own grid is the right default: rendering at the rate the
    // edit was authored against is the one output rate that needs no conform.
    let fps = options.fps.unwrap_or(project.timeline_fps);
    let settings = RenderSettings::new(options.resolution, fps)
        .with_bitrate(options.bitrate)
        .with_audio(options.sample_rate, options.audio_bitrate)
        .with_format(format);
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
    // Said every time, not only when a flag asked for it: the shape of the
    // file used to be an inference, and an inference nobody printed is one
    // nobody checks.
    println!("  format {format}");
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
