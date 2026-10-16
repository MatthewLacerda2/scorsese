//! `scorsese render`

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use scorsese_core::Fps;
use scorsese_core::style::Platform;
use scorsese_render::say;
use scorsese_render::{
    AudioCodec, Bands, Bitrate, Container, Cue, FrameRange, LoudnessTarget, RenderAsk,
    RenderSettings, Renderer, SampleRate, Sizing, Tools, VideoCodec, Workers, frames,
};

/// Everything the command line can say about the file to produce. Gathered into
/// one type rather than passed as seven positional arguments, where two
/// `Option<Bitrate>` next to each other are a bug waiting to be written.
pub(crate) struct Options {
    /// The canvas every layer is composited onto, and so the size of the file.
    /// `None` means the platform's size, or without one the project's own
    /// shape at delivery size (#1278) — which is also what a sound-only render
    /// takes, having no canvas to ask about. A shape (`9:16`) is spent at
    /// delivery size.
    /// Never given beside `platform`: clap refuses the pair, as the MCP
    /// `render` tool does, since both would size the render.
    pub(crate) resolution: Option<Sizing>,
    /// The placement the file is made for, which sizes it instead of
    /// `resolution`. `None` leaves the size to `resolution`.
    pub(crate) platform: Option<Platform>,
    /// `None` means the project's own timeline rate — the one output rate that
    /// conforms nothing.
    pub(crate) fps: Option<Fps>,
    /// `None` leaves the video encoder aiming for constant quality instead.
    pub(crate) bitrate: Option<Bitrate>,
    /// The rate the mix is produced at; sources recorded at others meet it.
    pub(crate) sample_rate: SampleRate,
    /// `None` leaves the audio encoder on its own default.
    pub(crate) audio_bitrate: Option<Bitrate>,
    /// `None` renders the whole timeline.
    pub(crate) range: Option<FrameRange>,
    /// `None` takes the container `out`'s extension asks for.
    pub(crate) container: Option<Container>,
    /// `None` takes the picture codec the container is written with.
    pub(crate) video_codec: Option<VideoCodec>,
    /// `None` takes the sound codec the container is written with.
    pub(crate) audio_codec: Option<AudioCodec>,
    /// How many threads composite at once. `None` leaves it to the machine —
    /// one fewer than it says it can run.
    pub(crate) threads: Option<u16>,
    /// Where to write PNG stills of the finished file. `None` writes none.
    pub(crate) stills: Option<PathBuf>,
    /// Which instants to still. Empty means every segment boundary, which is
    /// where a cut can be one frame wrong.
    pub(crate) at: Vec<Cue>,
    /// Whether a narration line not yet generated draws its band.
    pub(crate) bands: Bands,
    /// `None` delivers the mix as balanced; a target brings it there.
    pub(crate) loudness: Option<LoudnessTarget>,
}

/// What `--no-narration-bands` asks of a narration line not yet generated.
pub(crate) fn bands(no_narration_bands: bool) -> Bands {
    if no_narration_bands {
        Bands::Omitted
    } else {
        Bands::Drawn
    }
}

/// Renders the project to `out`, then prints what was written — for a headless
/// render those lines are the only report anyone gets.
pub(crate) fn run(project_dir: &Path, out: &Path, options: Options) -> Result<()> {
    // First, before the project is even opened: what shape the file is asked
    // to be is the cheapest thing to get wrong and the most expensive thing to
    // find out late. A combination we do not write is refused here, with
    // nothing spent and no ffmpeg yet located — by the rules every surface
    // shares (`RenderAsk`), so a flag means here what an argument means over
    // MCP and on the web.
    let ask = RenderAsk {
        container: options.container,
        video_codec: options.video_codec,
        audio_codec: options.audio_codec,
        resolution: options.resolution,
        platform: options.platform,
        fps: options.fps,
        bitrate: options.bitrate,
        sample_rate: Some(options.sample_rate),
        audio_bitrate: options.audio_bitrate,
        bands: options.bands,
        loudness: options.loudness,
    };
    let format = ask.format_for(out)?;
    ask.check(format)?;
    // The two flags only this command has, refused for a format with no
    // picture as the shared ones are, rather than ignored.
    for (given, setting) in [
        (options.threads.is_some(), "a compositing thread count"),
        (options.stills.is_some(), "stills"),
    ] {
        if given {
            format.picture_setting(setting)?;
        }
    }

    let project = super::open(project_dir)?;

    // The project's own grid and frame are the right defaults: rendering at
    // the rate the edit was authored against is the one output rate that
    // needs no conform, and at its own shape the one frame that crops nothing.
    let settings = ask.settings(format, &project)?;
    let range = options.range.unwrap_or(FrameRange::ALL);
    // A render plays whatever a synth asset was last baked to and never bakes
    // it again, so a recipe edited since, or a clip a song is fitted to that
    // has changed length, is said here rather than only heard (#1000).
    let stale = scorsese_providers::synth::out_of_date(&project, project_dir);

    let tools = Tools::discover()?;
    // Nothing about the file changes with the thread count — the same project
    // composites to the same frames however many workers draw them — so this
    // is the one render choice that never reaches `RenderSettings`.
    let mut renderer = Renderer::new(&tools, settings);
    if let Some(threads) = options.threads {
        renderer = renderer.with_workers(Workers::new(usize::from(threads)));
    }
    let report = renderer
        .render(&project, project_dir, range, out)
        .with_context(|| format!("rendering to {}", out.display()))?;

    println!("Wrote {} — {}", out.display(), say::written(&report));
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
        println!("  mix    {}", say::loudness(&levels.mix.whole.loudness));
        // Then the same statistics over time, because a soundtrack that sags
        // in its third minute reports one unremarkable average otherwise.
        for row in say::sections(&levels.mix) {
            println!("    {row}");
        }
        for (clip, level) in &levels.clips {
            println!("    {clip:<20} {}", say::loudness(level));
        }
    }
    if let Some(bitrate) = options.bitrate {
        println!("  bitrate {bitrate}");
    }
    // The file itself, read back — which is a different number from the mix
    // above whenever the codec is lossy, and the one a clipping verdict is
    // about. Then what was done to keep it under full scale, if anything was,
    // and a note for whatever the render could not do as asked.
    for line in say::report(&report, &stale) {
        println!("  {line}");
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
