//! `scorsese render`

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use scorsese_core::{Fps, Project};
use scorsese_render::say;
use scorsese_render::{
    AudioCodec, Bitrate, Container, Cue, FrameRange, OutputFormat, RenderSettings, Renderer,
    Resolution, SampleRate, Tools, VideoCodec, frames,
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
    /// Where to write PNG stills of the finished file. `None` writes none.
    pub stills: Option<PathBuf>,
    /// Which instants to still. Empty means every segment boundary, which is
    /// where a cut can be one frame wrong.
    pub at: Vec<Cue>,
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
    // Said every time, like the format above, and for the same reason. A mix
    // ten decibels too quiet passes every test in the suite; the only thing
    // that has ever caught one is a number printed when the render finishes.
    // It is a signal and never a gate — there is no correct loudness — so
    // nothing here can make the command fail.
    if let Some(levels) = &report.levels {
        println!("  mix    {}", say::loudness(&levels.mix));
        for (clip, level) in &levels.clips {
            println!("    {clip:<20} {}", say::loudness(level));
        }
    }
    if let Some(bitrate) = options.bitrate {
        println!("  bitrate {bitrate}");
    }
    for note in &report.notes {
        println!("  note: {note}");
    }
    // What is actually in the file, which until now nobody could learn without
    // watching it. Printed every time rather than behind a flag: an unattended
    // agent has no second chance to ask, and this cost nothing to produce.
    println!("{}", report.description);

    if let Some(directory) = &options.stills {
        stills(&tools, out, &options.at, &settings, &report, directory)?;
    }
    Ok(())
}

/// Writes the asked-for frames of the finished render out as PNGs.
///
/// The description says what the edit *claims*; these are what the pixels
/// actually did. That is the half no plan can check, because the plan is
/// derived from the same document that might be wrong about reality.
fn stills(
    tools: &Tools,
    out: &Path,
    at: &[Cue],
    settings: &RenderSettings,
    report: &scorsese_render::RenderReport,
    directory: &Path,
) -> Result<()> {
    let description = &report.description;
    let wanted: Vec<u64> = if at.is_empty() {
        description.boundaries()
    } else {
        at.iter().map(|cue| cue.out_frame(description)).collect()
    };
    let written = frames::stills(tools, out, &wanted, settings.resolution, directory)
        .with_context(|| format!("writing stills into {}", directory.display()))?;

    println!("  {} still(s) in {}", written.len(), directory.display());
    for still in &written {
        let seconds = description.from.seconds + still.frame as f64 / settings.fps.as_f64();
        println!(
            "    f{} ({seconds:.2}s) {}",
            still.frame,
            still.path.display()
        );
    }
    Ok(())
}
