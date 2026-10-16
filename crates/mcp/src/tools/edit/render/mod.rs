//! Encoding the timeline to a file.

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use scorsese_render::{Cancel, Renderer, Tools, say};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::renders::{Renders, Say, Work, watch};
use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Context, Costs, Reply, Tool};

mod settings;
#[cfg(test)]
mod tests;

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
    /// Output size, e.g. 1080x1920, or a shape like "9:16" at delivery size.
    /// Sources of another shape meet it the way each clip's fit says, and are
    /// never stretched. Default: the project's own shape at delivery size
    /// (1920x1080 for 16:9, 1080x1920 for 9:16).
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
    /// Frame rate to deliver at: 30, 60, or a fraction like 30000/1001 (a
    /// decimal like 29.97 is refused — write the fraction). Defaults to the
    /// project's own timeline rate; anything else is conformed from it,
    /// nearest frame. Refused for a sound-only container. (The CLI's --threads
    /// has no argument here: how many threads composite is a setting of the
    /// machine, not of the file, and this server leaves it to the machine.)
    fps: Option<String>,
    /// Target video bitrate, e.g. 8M or 800k — lower for a smaller file, as for
    /// sending over a chat app. Without it the encoder aims for constant
    /// quality instead. Refused for a sound-only container.
    bitrate: Option<String>,
    /// Sample rate the mix is produced at, in Hz: 48000 (or 48k), 44100 for
    /// the CD rate. Sources recorded at other rates are resampled to it.
    /// Default 48000.
    sample_rate: Option<String>,
    /// Target audio bitrate, e.g. 192k. Without it the encoder uses its own
    /// default, already transparent for speech and music.
    audio_bitrate: Option<String>,
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
         limiter holding its peaks. fps (60), bitrate (8M, for a smaller file), \
         sample_rate and audio_bitrate are scorsese render's flags of those names. \
         The reply says how loud the delivered file came out, and when the \
         soundtrack had to be turned down to keep a lossy codec from clipping, \
         and carries a note for each web page that could not be captured or \
         asked for something it was not given. It renders in the background: the answer is a job id at once, jobs \
         says how far it has got and what it wrote, and its cancel stops it. \
         Pass wait: true to answer only when the file is written instead; \
         started without it, jobs with job and wait: true waits for it."
    }

    // What it writes is its own output file, which the session's renders
    // already keep to one writer at a time; the project it only reads.
    fn writes(&self) -> bool {
        false
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
                 far it has got — add wait: true to answer only when it is done; when it \
                 is done, its line says what was written.",
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
    // to get wrong and the most expensive to find out late. Built by the
    // rules every surface shares, from `out` as the CLI builds it.
    let ask = settings::ask(arguments)?;
    let format = ask
        .format_for(&path)
        .map_err(|problem| problem.to_string())?;
    ask.check(format).map_err(|problem| problem.to_string())?;
    // Read now, so the render is of the cut as it stood when it was asked
    // for; an edit made while it runs is in the next one.
    let project = load(dir)?;

    let (settings, range) = settings::of(arguments, format, &project)?;

    // Discovered per call rather than held: a server that found ffmpeg at
    // startup would keep insisting it was there after someone uninstalled
    // it, and this is not a hot path.
    let tools = Tools::discover().map_err(|error| format!("{error}"))?;
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
        // Then what the CLI prints after its file, in its words: how loud the
        // file came out and whether it had to be turned down to get there,
        // then a note for a page that could not be captured (#777) or a bake
        // played out of date (#1000). An agent is the caller least able to
        // hear the result for itself, and cannot fix what it is not told.
        let mut words = format!("wrote {said} — {}, as {format}", say::written(&report));
        for line in say::report(&report, &stale) {
            words.push_str(&format!("\n{line}"));
        }
        Ok(words)
    });
    Ok((out, path, work))
}
