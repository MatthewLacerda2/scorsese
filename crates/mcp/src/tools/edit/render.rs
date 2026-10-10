//! Encoding the timeline to a file.

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use scorsese_core::style::Platform;
use scorsese_render::{
    AudioCodec, Bands, Cancel, Container, FrameRange, LoudnessTarget, OutputFormat, RenderSettings,
    Renderer, Resolution, Tools, VideoCodec, say,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::renders::{Renders, Say, Work, watch};
use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Context, Costs, Reply, Tool};

/// Encode the timeline to a file.
pub(crate) struct Render;

/// What `render` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Where to write the file, e.g. teaser.mp4, or score.mp3 for the
    /// soundtrack alone. The extension chooses the container unless `container`
    /// names one: mp4, mkv, avi or wmv for video; mp3, wav or m4a for sound
    /// only. Any other extension, or none, is refused. A relative path is
    /// relative to the project directory, never the server's working directory;
    /// an absolute one is used as given.
    out: String,
    /// Container to deliver in: mp4, mkv, avi or wmv for video; mp3, wav or m4a
    /// for sound only. Defaults to what `out`'s extension asks for, so naming
    /// the file is usually the whole decision.
    container: Option<String>,
    /// Picture codec: h264, mpeg4 or wmv2. Defaults to what the container is
    /// written with — h264 for mp4 and mkv, mpeg4 for avi, wmv2 for wmv. A
    /// pairing scorsese does not write is refused before anything is encoded,
    /// and so is any video_codec for a sound-only container.
    video_codec: Option<String>,
    /// Sound codec: aac, pcm_s16le, wmav2 or mp3. Defaults, like video_codec, to
    /// what the container is written with — aac for mp4, mkv and m4a, pcm_s16le
    /// for avi and wav, wmav2 for wmv, mp3 for mp3.
    audio_codec: Option<String>,
    /// Output size, e.g. 1920x1080. Sources of another shape meet it the way
    /// each clip's fit says, and are never stretched. Default 1920x1080.
    /// Refused for a sound-only container, which has no picture to size.
    resolution: Option<String>,
    /// The placement the video is delivered for, which sizes it: youtube
    /// (1920x1080), or youtube_shorts, instagram_reels, instagram_reels_ad,
    /// instagram_stories_ad, tiktok or tiktok_ad (1080x1920). A preset for this
    /// render only; the project is not changed. Cannot be given with
    /// resolution, and refused for a sound-only container.
    platform: Option<String>,
    /// true to answer only when the file is written, the way a short render you
    /// need before your next step is best asked for. Default false: the render
    /// runs in the background and the answer is its job id, so you can tell the
    /// person how far it has got with jobs while it runs. A waited call sends
    /// MCP progress notifications when it carries a progressToken.
    #[serde(default)]
    wait: bool,
    /// Render only part of the timeline, in frames: 30:120 covers frames 30 up
    /// to 120, 30: runs to the end, :120 from the start. Without it the whole
    /// timeline is rendered.
    range: Option<String>,
    /// false to leave out the band a narration line not yet generated draws
    /// across the foot of the picture: for a preview whose captions already
    /// carry the words. The line keeps its place on the timeline, stays
    /// silent until generated and still ducks the music, and the reply names
    /// every line left out. Default true, every band drawn. Refused for a
    /// sound-only container.
    narration_bands: Option<bool>,
    /// Deliver the soundtrack at this integrated loudness, in LUFS, e.g. -14:
    /// the mix is measured, raised or lowered to it, and its peaks held under
    /// -1 dBTP by a limiter, so a quiet mix arrives as loud as the feed around
    /// it. Between -40 and -5. Without it the mix is delivered as balanced,
    /// turned down only as far as a lossy codec needs. The reply gives the
    /// loudness before and after, and the gain and limiting it took.
    loudness: Option<f64>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("out", "where to write the file")];
}

impl Tool for Render {
    fn name(&self) -> &'static str {
        "render"
    }

    fn description(&self) -> &'static str {
        "Render the timeline to a video file, or to a sound file of its mix \
         alone. Needs ffmpeg and takes real time — call project_describe first \
         to check the cut is right, since that costs nothing. An out ending in \
         .mp3, .wav or .m4a delivers the soundtrack with no picture, and never \
         composites a frame. Sketch and stale generated assets render as slug \
         cards rather than failing, so a preview cut always produces something; \
         narration_bands: false leaves the narration lines' bands off the picture. \
         platform: tiktok (or youtube, instagram_reels…) sizes it for that \
         placement. loudness: -14 (any LUFS) brings the soundtrack to that loudness, with a \
         limiter holding its peaks. \
         The reply says how loud the delivered file came out, and when the \
         soundtrack had to be turned down to keep a lossy codec from clipping, \
         and carries a note for each web page that could not be captured or \
         asked for something it was not given. It renders in the background: the answer is a job id at once, jobs \
         says how far it has got and what it wrote, and its cancel stops it. \
         Pass wait: true to answer only when the file is written instead."
    }

    fn costs(&self) -> Costs {
        Costs::Encode
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    /// Outside a session nobody could ask after a render later, so it waits.
    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        self.call_cancellable(arguments, &Cancel::new())
    }

    /// Waits, for the reason [`Render::call`] does, and stops between frames
    /// when `cancel` is tripped.
    fn call_cancellable(&self, arguments: &Value, cancel: &Cancel) -> Result<Reply, String> {
        let renders = Renders::default();
        let mut waited = arguments.clone();
        if let Some(fields) = waited.as_object_mut() {
            fields.insert("wait".to_owned(), json!(true));
        }
        self.call_in(&waited, &mut Context::new(cancel, &renders, None))
    }

    /// Starts the render as one of the session's jobs and answers with its id
    /// — or, with `wait`, answers when the file is written, reporting progress
    /// on the way when the client asked for it.
    ///
    /// Everything that can be refused is refused here, before the job starts,
    /// so a refusal is the call's answer rather than a job that failed at once.
    /// A waited render stops when the client cancels the call (#647) and
    /// removes the file it had begun; the words it would have answered with —
    /// how far it got — are the server's to log, since a cancelled request is
    /// not answered.
    fn call_in(&self, arguments: &Value, context: &mut Context<'_>) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let (out, path, work) = prepared(dir, &arguments)?;
        if !arguments.wait {
            let job = context
                .renders
                .start(dir, (&out, path), Cancel::new(), work)?;
            return Ok(format!(
                "Rendering {out} as job {id}, running. Call jobs with job: {id} to see how \
                 far it has got; when it is done, its line says what was written.",
                id = job.id
            )
            .into());
        }
        let job = context
            .renders
            .start(dir, (&out, path), context.cancel.clone(), work)?;
        let report = context
            .report
            .as_mut()
            .map(|report| &mut **report as &mut Say<'_>);
        watch(&job, context.cancel, report).map(Reply::from)
    }
}

/// The render `arguments` ask for, checked and ready to run: the words its
/// answers use for `out`, the path it writes, and the work itself.
fn prepared(dir: &Path, arguments: &Arguments) -> Result<(String, PathBuf, Work), String> {
    // Against the project, not the server's working directory, which
    // belongs to whoever launched it (#518). The caller's own string is
    // what the reply says back, because that is the path the next call —
    // `hear` — resolves the same way.
    let path = args::path(dir, &arguments.out, "out")?;
    let out = arguments.out.clone();
    // First, before the project is opened — the order `scorsese render`
    // keeps, for its reason: the shape of the file is the cheapest thing
    // to get wrong and the most expensive to find out late.
    let format = format(arguments, &path)?;
    // Read now, so the render is of the cut as it stood when it was asked
    // for; an edit made while it runs is in the next one.
    let project = load(dir)?;

    // Refused for a format with no picture rather than ignored, in the
    // words `scorsese render --resolution` is refused in.
    let resolution = match (&arguments.resolution, &arguments.platform) {
        (Some(_), Some(_)) => {
            return Err("resolution and platform both size the render; give one".to_owned());
        }
        (Some(text), None) => {
            format
                .picture_setting("a resolution")
                .map_err(|problem| format!("{problem}"))?;
            text.parse()
                .map_err(|problem| format!("resolution: {problem}"))?
        }
        // In `scorsese render --platform`'s words, refusal included.
        (None, Some(text)) => {
            format
                .picture_setting("a platform")
                .map_err(|problem| format!("{problem}"))?;
            Resolution::of(
                text.parse::<Platform>()
                    .map_err(|problem| format!("platform: {problem}"))?,
            )
        }
        (None, None) => Resolution::HD,
    };

    // The CLI's `--no-narration-bands`, refused for a format with no picture
    // in the same words.
    let bands = match arguments.narration_bands {
        Some(false) => {
            format
                .picture_setting("leaving narration bands out")
                .map_err(|problem| format!("{problem}"))?;
            Bands::Omitted
        }
        Some(true) | None => Bands::Drawn,
    };

    // The CLI's `--loudness`, held to the same range in the same words.
    let loudness = arguments
        .loudness
        .map(LoudnessTarget::new)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;

    // The CLI's `--range`, parsed by the CLI's parser: `FrameRange`'s own
    // `FromStr` is the one set of rules, so `30:`, `:120` and every refusal
    // read the same from either client. Parsed before ffmpeg is looked for,
    // because a range that is not one costs nothing to refuse.
    let range = match &arguments.range {
        Some(text) => text
            .parse()
            .map_err(|problem| format!("range: {problem}"))?,
        None => FrameRange::ALL,
    };

    // Discovered per call rather than held: a server that found ffmpeg at
    // startup would keep insisting it was there after someone uninstalled
    // it, and this is not a hot path.
    let tools = Tools::discover().map_err(|error| format!("{error}"))?;
    // The project's own grid by default: rendering at the rate the edit
    // was authored against is the one output rate needing no conform.
    let settings = RenderSettings::new(resolution, project.timeline_fps)
        .with_format(format)
        .with_bands(bands)
        .with_loudness(loudness);
    // What `scorsese render` prints as a note, in its words: a synth asset
    // that would play a bake older than its recipe or its clip (#1000).
    let stale = scorsese_providers::synth::out_of_date(&project, dir);
    let (dir, target, said) = (dir.to_owned(), path.clone(), out.clone());
    let work: Work = Box::new(move |progress, cancel| {
        let report = Renderer::new(&tools, settings)
            .with_cancel(cancel)
            .with_progress(progress)
            .render(&project, &dir, range, &target)
            .map_err(|error| format!("rendering: {error}"))?;
        // Then what the CLI prints about sound, in its words: how loud the
        // file came out, and whether it had to be turned down to get there.
        // An agent is the caller least able to hear the result for itself.
        let mut words = format!("wrote {said} — {}, as {format}", say::written(&report));
        for line in say::delivery(&report) {
            words.push_str(&format!("\n{line}"));
        }
        // What the CLI prints as `note:` lines, in the same words: a page
        // that could not be captured, or one that asked for something it was
        // not given (#777). An agent cannot fix what it is not told.
        for note in &report.notes {
            words.push_str(&format!("\nnote: {note}"));
        }
        for note in &stale {
            words.push_str(&format!("\nnote: {note}"));
        }
        Ok(words)
    });
    Ok((out, path, work))
}

/// The shape of the file, built the way `scorsese render` builds it — by
/// [`OutputFormat::for_path`] — so a file name, an override and every refusal
/// read the same from either client. Checked before the project's media or
/// ffmpeg are touched, because a combination we do not write costs nothing to
/// refuse and a whole encode to discover.
fn format(arguments: &Arguments, out: &Path) -> Result<OutputFormat, String> {
    let container = arguments
        .container
        .as_deref()
        .map(str::parse::<Container>)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;
    let video = arguments
        .video_codec
        .as_deref()
        .map(str::parse::<VideoCodec>)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;
    let audio = arguments
        .audio_codec
        .as_deref()
        .map(str::parse::<AudioCodec>)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;
    OutputFormat::for_path(out, container, video, audio).map_err(|problem| format!("{problem}"))
}
