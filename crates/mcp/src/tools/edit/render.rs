//! Encoding the timeline to a file.

use std::path::Path;

use scorsese_render::{
    AudioCodec, Container, FrameRange, OutputFormat, RenderSettings, Renderer, Resolution, Tools,
    VideoCodec, say,
};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property, under};

/// Encode the timeline to a file.
pub(crate) struct Render;

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
         cards rather than failing, so a preview cut always produces something. \
         The reply says how loud the delivered file came out, and when the \
         soundtrack had to be turned down to keep a lossy codec from clipping."
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
                    "description": "Where to write the file, e.g. teaser.mp4, or \
                                    score.mp3 for the soundtrack alone. The \
                                    extension chooses the container unless \
                                    `container` names one: mp4, mkv, avi or wmv \
                                    for video; mp3, wav or m4a for sound only. \
                                    Any other extension, or none, is refused. \
                                    A relative path is relative to the project \
                                    directory, never the server's working \
                                    directory; an absolute one is used as given."
                },
                "container": {
                    "type": "string",
                    "description": "Container to deliver in: mp4, mkv, avi or wmv \
                                    for video; mp3, wav or m4a for sound only. \
                                    Defaults to what `out`'s extension asks for, so \
                                    naming the file is usually the whole decision."
                },
                "video_codec": {
                    "type": "string",
                    "description": "Picture codec: h264, mpeg4 or wmv2. Defaults to \
                                    what the container is written with — h264 for \
                                    mp4 and mkv, mpeg4 for avi, wmv2 for wmv. A \
                                    pairing scorsese does not write is refused \
                                    before anything is encoded, and so is any \
                                    video_codec for a sound-only container."
                },
                "audio_codec": {
                    "type": "string",
                    "description": "Sound codec: aac, pcm_s16le, wmav2 or mp3. \
                                    Defaults, like video_codec, to what the \
                                    container is written with — aac for mp4, mkv \
                                    and m4a, pcm_s16le for avi and wav, wmav2 for \
                                    wmv, mp3 for mp3."
                },
                "resolution": {
                    "type": "string",
                    "description": "Output size, e.g. 1920x1080. Sources of another \
                                    shape meet it the way each clip's fit says, and \
                                    are never stretched. Default 1920x1080. Refused \
                                    for a sound-only container, which has no \
                                    picture to size."
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
        // Against the project, not the server's working directory, which
        // belongs to whoever launched it (#518). The caller's own string is
        // what the reply says back, because that is the path the next call —
        // `audio_level`, `hear` — resolves the same way.
        let path = under(&dir, arguments, "out")?
            .ok_or_else(|| "`out` is required: where to write the file".to_owned())?;
        let out = arguments
            .get("out")
            .and_then(Value::as_str)
            .unwrap_or_default();
        // First, before the project is opened — the order `scorsese render`
        // keeps, for its reason: the shape of the file is the cheapest thing
        // to get wrong and the most expensive to find out late.
        let format = format(arguments, &path)?;
        let project = load(&dir)?;

        // Refused for a format with no picture rather than ignored, in the
        // words `scorsese render --resolution` is refused in.
        let resolution = match arguments.get("resolution").and_then(Value::as_str) {
            Some(text) => {
                format
                    .picture_setting("a resolution")
                    .map_err(|problem| format!("{problem}"))?;
                text.parse()
                    .map_err(|problem| format!("resolution: {problem}"))?
            }
            None => Resolution::HD,
        };

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
            .render(&project, &dir, range, &path)
            .map_err(|error| format!("rendering: {error}"))?;
        // Then what the CLI prints about sound, in its words: how loud the file
        // came out, and whether it had to be turned down to get there. An agent
        // is the caller least able to hear the result for itself.
        let mut said = format!("wrote {out} — {}, as {format}", say::written(&report));
        for line in say::delivery(&report) {
            said.push_str(&format!("\n{line}"));
        }
        Ok(said.into())
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
