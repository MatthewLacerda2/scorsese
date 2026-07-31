//! One frame of the timeline, as a picture the client can look at.
//!
//! The tool that closes the loop the rest of this server only describes.
//! Everything else here answers in words — what the document says, what the cut
//! contains, what is wrong with it — and words are inference: an assistant that
//! writes a title and reads back "TEASER, centred, 0.5s" still has no idea
//! whether it is readable, whether it collides with the shot under it, or
//! whether it is on screen at all. This is the call that shows it.
//!
//! It is the same picture the render delivers, because it comes from the same
//! code: [`Renderer::still`] is the render pipeline with the encoder taken out.
//! A still drawn any other way could disagree with the file, and then looking
//! at it would prove nothing.

use std::path::PathBuf;

use scorsese_core::Fps;
use scorsese_render::{Cue, RenderSettings, Renderer, Resolution, Tools, frames};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Reply, Tool, project_dir, project_property};

/// What the frame is composited at when nobody says.
///
/// Smaller than a delivery raster on purpose, and it costs nothing in fidelity:
/// everything a title or a layout is placed by is a fraction of the frame, so
/// 1280x720 is the same picture as 1920x1080 with fewer pixels in it. What it
/// saves is the wire — an image block is base64 inside one JSON line, and a
/// client asking for a frame while it works should not be sending megabytes to
/// see a cut. A caller who wants the delivery raster asks for it.
const DEFAULT_RASTER: &str = "1280x720";

/// A frame, composited and handed back as a picture.
pub(crate) struct Still;

impl Tool for Still {
    fn name(&self) -> &'static str {
        "still"
    }

    fn description(&self) -> &'static str {
        "Look at one frame of the edit. Composites the timeline at one instant \
         and returns the picture itself — the same pixels a render would \
         deliver, since it is the render pipeline with the encoder taken out. \
         Needs ffmpeg, but encodes nothing: seconds, not a whole render. Use it \
         to check what project_describe can only assert — that a title is \
         readable, that a layer is where it was meant to be, that a cut lands. \
         Sketch and stale generated assets appear as slug cards, so a frame of \
         an unrealised shot still shows something."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "at": {
                    "type": "string",
                    "description": "Which instant to look at: a time like 9.1s, or a \
                                    timeline frame number like 285. A bare decimal is \
                                    refused — say which unit you mean."
                },
                "resolution": {
                    "type": "string",
                    "description": "The raster to composite at, e.g. 1920x1080. Layout \
                                    is a fraction of the frame, so a smaller one is the \
                                    same picture and a smaller reply. Default 1280x720."
                },
                "out": {
                    "type": "string",
                    "description": "Also keep the PNG at this path, e.g. review/title.png. \
                                    Without it the picture is returned and nothing is \
                                    left on disk."
                }
            },
            "required": ["project", "at"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let at = instant(arguments, project.timeline_fps)?;
        let resolution: Resolution = arguments
            .get("resolution")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_RASTER)
            .parse()
            .map_err(|problem| format!("resolution: {problem}"))?;

        // Discovered per call rather than held, as `render` does: a server that
        // found ffmpeg at startup would keep insisting it was there after
        // someone uninstalled it.
        let tools = Tools::discover().map_err(|error| format!("{error}"))?;
        // The project's own grid, so the frame handed back is the frame asked
        // for rather than the nearest one at some other rate.
        let settings = RenderSettings::new(resolution, project.timeline_fps);
        let frame = Renderer::new(&tools, settings)
            .still(&project, &dir, at)
            .map_err(|error| format!("compositing frame {}: {error}", at.get()))?;

        // Written to a file either way: PNG encoding is ffmpeg's, and ffmpeg
        // writes files. Where it goes is the only difference — a path the
        // caller named, kept, or a scratch file that is read back and removed.
        let kept = arguments.get("out").and_then(Value::as_str);
        let png = Scratch::at(kept);
        frames::write_png(&tools, &png.path, &frame)
            .map_err(|error| format!("writing the frame: {error}"))?;
        let bytes = std::fs::read(&png.path)
            .map_err(|error| format!("reading {} back: {error}", png.path.display()))?;

        let seconds = project.timeline_fps.seconds(at);
        let mut said = format!(
            "frame {} ({seconds:.2}s) of {} at {resolution}",
            at.get(),
            project.name
        );
        if let Some(path) = kept {
            said.push_str(&format!(" — written to {path}"));
        }
        Ok(Reply::picture(said, &bytes))
    }
}

/// Which timeline frame the `at` argument names.
fn instant(arguments: &Value, fps: Fps) -> Result<scorsese_core::Frames, String> {
    let at = arguments
        .get("at")
        .and_then(Value::as_str)
        .ok_or_else(|| "`at` is required: a time like 9.1s, or a frame like 285".to_owned())?;
    at.parse::<Cue>()
        .map(|cue| cue.timeline_frame(fps))
        .map_err(|problem| format!("at: {problem}"))
}

/// Where the PNG is written, and whether it survives the call.
///
/// A caller who named a path gets the file and keeps it. A caller who did not
/// still needs one written somewhere, because the encoding is ffmpeg's and
/// ffmpeg writes files — so it goes to a scratch path that is removed on the
/// way out, including when the call fails partway through. A server left
/// littering the temporary directory with frames is a server nobody notices
/// filling a disk.
struct Scratch {
    path: PathBuf,
    remove: bool,
}

impl Scratch {
    fn at(kept: Option<&str>) -> Self {
        match kept {
            Some(path) => Self {
                path: PathBuf::from(path),
                remove: false,
            },
            None => Self {
                path: std::env::temp_dir().join(format!(
                    "scorsese-still-{}-{}.png",
                    std::process::id(),
                    unique()
                )),
                remove: true,
            },
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.remove {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// A number no other call in this process has used, so two frames asked for at
/// once cannot land on one scratch file.
fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
