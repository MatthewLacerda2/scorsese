//! Encoding the timeline to a file.

use std::path::Path;

use scorsese_render::{
    AudioCodec, Container, FrameRange, OutputFormat, RenderSettings, Renderer, Resolution, Tools,
    VideoCodec,
};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Encode the timeline to a file.
pub(crate) struct Render;

impl Tool for Render {
    fn name(&self) -> &'static str {
        "render"
    }

    fn description(&self) -> &'static str {
        "Render the timeline to a video file. Needs ffmpeg and takes real time \
         — call project_describe first to check the cut is right, since that \
         costs nothing. Sketch and stale generated assets render as slug cards \
         rather than failing, so a preview cut always produces something."
    }

    fn costs(&self) -> Costs {
        Costs::Encode
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "out": {
                    "type": "string",
                    "description": "Where to write the file, e.g. teaser.mp4. The \
                                    extension chooses the container unless \
                                    `container` names one: mp4, mkv, avi or wmv. \
                                    Any other extension, or none, is refused."
                },
                "container": {
                    "type": "string",
                    "description": "Container to deliver in: mp4, mkv, avi or wmv. \
                                    Defaults to what `out`'s extension asks for, so \
                                    naming the file is usually the whole decision."
                },
                "video_codec": {
                    "type": "string",
                    "description": "Picture codec: h264, mpeg4 or wmv2. Defaults to \
                                    what the container is written with — h264 for \
                                    mp4 and mkv, mpeg4 for avi, wmv2 for wmv. A \
                                    pairing scorsese does not write is refused \
                                    before anything is encoded."
                },
                "audio_codec": {
                    "type": "string",
                    "description": "Sound codec: aac, pcm_s16le or wmav2. Defaults, \
                                    like video_codec, to what the container is \
                                    written with — aac for mp4 and mkv, pcm_s16le \
                                    for avi, wmav2 for wmv."
                },
                "resolution": {
                    "type": "string",
                    "description": "Output size, e.g. 1920x1080. Sources of another \
                                    shape meet it the way each clip's fit says, and \
                                    are never stretched. Default 1920x1080."
                },
                "range": {
                    "type": "string",
                    "description": "Render only part of the timeline, in frames: \
                                    30:120 covers frames 30 up to 120, 30: runs to \
                                    the end, :120 from the start. Without it the \
                                    whole timeline is rendered."
                }
            },
            "required": ["project", "out"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let out = arguments
            .get("out")
            .and_then(Value::as_str)
            .ok_or_else(|| "`out` is required: where to write the file".to_owned())?;
        // First, before the project is opened — the order `scorsese render`
        // keeps, for its reason: the shape of the file is the cheapest thing
        // to get wrong and the most expensive to find out late.
        let format = format(arguments, Path::new(out))?;
        let project = load(&dir)?;

        let resolution: Resolution = arguments
            .get("resolution")
            .and_then(Value::as_str)
            .unwrap_or("1920x1080")
            .parse()
            .map_err(|problem| format!("resolution: {problem}"))?;

        // The CLI's `--range`, parsed by the CLI's parser: `FrameRange`'s own
        // `FromStr` is the one set of rules, so `30:`, `:120` and every refusal
        // read the same from either client. Parsed before ffmpeg is looked for,
        // because a range that is not one costs nothing to refuse.
        let range = match arguments.get("range").and_then(Value::as_str) {
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
        let settings = RenderSettings::new(resolution, project.timeline_fps).with_format(format);
        let report = Renderer::new(&tools, settings)
            .render(&project, &dir, range, Path::new(out))
            .map_err(|error| format!("rendering: {error}"))?;
        Ok(format!(
            "wrote {out} — {} frames at {} fps, {} ({:.2}s), as {format}",
            report.frames,
            settings.fps,
            report.resolution,
            report.seconds()
        )
        .into())
    }
}

/// The shape of the file, built the way `scorsese render` builds it — by
/// [`OutputFormat::for_path`] — so a file name, an override and every refusal
/// read the same from either client. Checked before the project's media or
/// ffmpeg are touched, because a combination we do not write costs nothing to
/// refuse and a whole encode to discover.
fn format(arguments: &Value, out: &Path) -> Result<OutputFormat, String> {
    let named = |key: &str| arguments.get(key).and_then(Value::as_str);
    let container = named("container")
        .map(str::parse::<Container>)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;
    let video = named("video_codec")
        .map(str::parse::<VideoCodec>)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;
    let audio = named("audio_codec")
        .map(str::parse::<AudioCodec>)
        .transpose()
        .map_err(|problem| format!("{problem}"))?;
    OutputFormat::for_path(out, container, video, audio).map_err(|problem| format!("{problem}"))
}
