//! Encoding the timeline to a file.

use std::path::Path;

use scorsese_render::{FrameRange, RenderSettings, Renderer, Resolution, Tools};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Reply, Tool, project_dir, project_property};

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

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "out": {
                    "type": "string",
                    "description": "Where to write the file, e.g. teaser.mp4. The \
                                    extension chooses the container."
                },
                "resolution": {
                    "type": "string",
                    "description": "Output size, e.g. 1920x1080. Sources of another \
                                    shape meet it the way each clip's fit says, and \
                                    are never stretched. Default 1920x1080."
                }
            },
            "required": ["project", "out"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let out = arguments
            .get("out")
            .and_then(Value::as_str)
            .ok_or_else(|| "`out` is required: where to write the file".to_owned())?;

        let resolution: Resolution = arguments
            .get("resolution")
            .and_then(Value::as_str)
            .unwrap_or("1920x1080")
            .parse()
            .map_err(|problem| format!("resolution: {problem}"))?;

        // Discovered per call rather than held: a server that found ffmpeg at
        // startup would keep insisting it was there after someone uninstalled
        // it, and this is not a hot path.
        let tools = Tools::discover().map_err(|error| format!("{error}"))?;
        // The project's own grid by default: rendering at the rate the edit
        // was authored against is the one output rate needing no conform.
        let settings = RenderSettings::new(resolution, project.timeline_fps);
        let report = Renderer::new(&tools, settings)
            .render(&project, &dir, FrameRange::ALL, Path::new(out))
            .map_err(|error| format!("rendering: {error}"))?;
        Ok(format!(
            "wrote {out} — {} frames at {} fps, {} ({:.2}s)",
            report.frames,
            settings.fps,
            report.resolution,
            report.seconds()
        )
        .into())
    }
}
