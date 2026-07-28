//! Tools that change something: write the document, duck the music, render.

use std::path::Path;

use scorsese_core::{Dip, Frames, Project, PropertyPath, TrackId, Under, duck_track};
use scorsese_render::audio::path::VOLUME;
use scorsese_render::{FrameRange, RenderSettings, Renderer, Resolution, Tools};
use serde_json::Value;

use super::inspect::load;
use super::{Tool, project_dir, project_property};

/// Replace the project document.
pub(super) struct Write;

impl Tool for Write {
    fn name(&self) -> &'static str {
        "project_write"
    }

    fn description(&self) -> &'static str {
        "Replace a project's project.json with the document given. The whole \
         edit is this file, so this is how any change is made: read it, change \
         it, write it back. **Validated before it is written** — a document \
         that would not load is refused with every problem listed, and the file \
         on disk is left exactly as it was."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "document": {
                    "type": "string",
                    "description": "The complete project.json to write, as text. \
                                    Not a patch — whatever is here replaces the file."
                }
            },
            "required": ["project", "document"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<String, String> {
        let dir = project_dir(arguments)?;
        let document = arguments
            .get("document")
            .and_then(Value::as_str)
            .ok_or_else(|| "`document` is required: the project.json to write".to_owned())?;

        // Parsed *and* validated before anything is written. An editor may save
        // work that is temporarily incoherent — a person with the file open —
        // but a tool writing a whole document has no such excuse, and a broken
        // project.json is the one thing that makes every other tool useless.
        let project = Project::from_json(document)
            .map_err(|problem| format!("refused, nothing written: {problem}"))?;
        project
            .validate()
            .map_err(|problems| format!("refused, nothing written:\n{problems}"))?;

        project
            .save(&dir)
            .map_err(|error| format!("writing the project: {error}"))?;
        Ok(format!(
            "written: {} asset(s), {} track(s), {} clip(s)",
            project.assets.len(),
            project.tracks.len(),
            project.clips().count()
        ))
    }
}

/// Lower the music under narration.
pub(super) struct Duck;

impl Tool for Duck {
    fn name(&self) -> &'static str {
        "duck_music"
    }

    fn description(&self) -> &'static str {
        "Lower a music track while narration plays over it, by writing ordinary \
         volume keyframes on its clips. Safe to run repeatedly: it replaces only \
         the keyframes it wrote and never touches ones set by hand. Works on \
         narration that has not been generated yet, since it triggers on where \
         the clips are rather than on the sound."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "music": {
                    "type": "string",
                    "description": "Id of the audio track to duck — the music."
                },
                "depth": {
                    "type": "number",
                    "description": "How far down, as a multiplier on the clip's own \
                                    level. 0.25 is a quarter as loud. Default 0.25."
                },
                "attack_seconds": {
                    "type": "number",
                    "description": "Seconds to reach the ducked level. The dip is \
                                    fully down by the moment narration starts. \
                                    Default 0.3."
                },
                "release_seconds": {
                    "type": "number",
                    "description": "Seconds to come back up. Longer than the attack \
                                    on purpose — returning early lurches. Default 0.6."
                },
                "under": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Track ids that count as narration. Omit and every \
                                    other audio track does."
                }
            },
            "required": ["project", "music"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<String, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let music = arguments
            .get("music")
            .and_then(Value::as_str)
            .ok_or_else(|| "`music` is required: the track id to duck".to_owned())?;

        let fps = project.timeline_fps.as_f64();
        let dip = Dip {
            under: number(arguments, "depth", 0.25),
            over: 1.0,
            attack: frames(number(arguments, "attack_seconds", 0.3), fps),
            release: frames(number(arguments, "release_seconds", 0.6), fps),
        };
        let under = Under {
            music: TrackId::new(music),
            narration: arguments
                .get("under")
                .and_then(Value::as_array)
                .map(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .map(TrackId::new)
                        .collect()
                })
                .unwrap_or_default(),
        };

        let report = duck_track(&mut project, &under, &PropertyPath::new(VOLUME), dip)
            .ok_or_else(|| format!("no track `{music}` in this project"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(format!(
            "{} clip(s) ducked, {} left alone — ordinary volume keyframes, \
             editable and deletable",
            report.dipped.len(),
            report.untouched.len()
        ))
    }
}

/// Encode the timeline to a file.
pub(super) struct Render;

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

    fn call(&self, arguments: &Value) -> Result<String, String> {
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
        ))
    }
}

/// A number argument, or its default.
fn number(arguments: &Value, key: &str, fallback: f64) -> f64 {
    arguments
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(fallback)
}

/// Seconds on the project's grid, at least one frame when anything was asked
/// for — a ramp rounded to no frames is a switch, not a ramp.
fn frames(seconds: f64, fps: f64) -> Frames {
    if seconds <= 0.0 {
        return Frames::ZERO;
    }
    Frames(((seconds * fps).round() as u64).max(1))
}
