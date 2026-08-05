//! Frames of the timeline, as pictures the client can look at.
//!
//! The tool that closes the loop the rest of this server only describes.
//! Everything else here answers in words — what the document says, what the cut
//! contains, what is wrong with it — and words are inference: an assistant that
//! writes a title and reads back "TEASER, centred, 0.5s" still has no idea
//! whether it is readable, whether it collides with the shot under it, or
//! whether it is on screen at all. This is the call that shows it.
//!
//! It takes several instants because *"does every section look right?"* is one
//! question, and a tool that answers one frame at a time turns it into a round
//! trip per section. A picture is the most expensive reply this server sends,
//! so the cost of looking is what decides how often anything gets verified —
//! and an assistant that checks one of six sections and reports on all six
//! fails without erroring.
//!
//! It is the same picture the render delivers, because it comes from the same
//! code: [`Renderer::still`] is the render pipeline with the encoder taken out.
//! A still drawn any other way could disagree with the file, and then looking
//! at it would prove nothing.

use scorsese_core::{Fps, Frames};
use scorsese_render::{Cue, RenderSettings, Renderer, Resolution, Tools, frames, grid};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::scratch::Scratch;
use crate::tools::{Costs, Part, Reply, Tool, project_dir, project_property};

/// What the frame is composited at when nobody says.
///
/// Smaller than a delivery raster on purpose, and it costs nothing in fidelity:
/// everything a title or a layout is placed by is a fraction of the frame, so
/// 1280x720 is the same picture as 1920x1080 with fewer pixels in it. What it
/// saves is the wire — an image block is base64 inside one JSON line, and a
/// client asking for a frame while it works should not be sending megabytes to
/// see a cut. A caller who wants the delivery raster asks for it.
const DEFAULT_RASTER: &str = "1280x720";

/// Frames, composited and handed back as pictures.
pub(crate) struct Still;

impl Tool for Still {
    fn name(&self) -> &'static str {
        "still"
    }

    fn description(&self) -> &'static str {
        "Look at the edit. Composites the timeline at one instant, or at a \
         whole list of them, and returns the pictures themselves — the same \
         pixels a render would deliver, since it is the render pipeline with \
         the encoder taken out. One sentence and one picture comes back per \
         instant, in the order asked, so checking every section of a cut is \
         one call rather than one per section. Needs ffmpeg, but encodes \
         nothing: seconds, not a whole render. Use it to check what \
         project_describe can only assert — that a title is readable, that a \
         layer is where it was meant to be, that a cut lands. Sketch and stale \
         generated assets appear as slug cards, so a frame of an unrealised \
         shot still shows something. Pass grid: true to have the frame ruled in \
         the fractions the document itself takes, so a coordinate is read off \
         the picture rather than converged on by guessing."
    }

    fn costs(&self) -> Costs {
        Costs::Frames
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "at": {
                    "type": ["string", "array"],
                    "items": { "type": "string" },
                    "description": "Which instant to look at: a time like 9.1s, or a \
                                    timeline frame number like 285. A bare decimal is \
                                    refused — say which unit you mean. Give a list, e.g. \
                                    [\"0s\", \"9.1s\", \"400\"], to look at several at \
                                    once: one sentence and one picture comes back per \
                                    instant, in the order asked."
                },
                "resolution": {
                    "type": "string",
                    "description": "The raster to composite at, e.g. 1920x1080. Layout \
                                    is a fraction of the frame, so a smaller one is the \
                                    same picture and a smaller reply. Default 1280x720."
                },
                "grid": {
                    "type": "boolean",
                    "description": "Rule the picture with coordinates: a line every 0.1 of \
                                    the frame, heavier at 0.5, labelled along the top and \
                                    left edges, origin at the top-left corner. Fractions of \
                                    the raster, which is the unit transform.position.x and \
                                    transform.position.y are written in — so where a layer \
                                    sits is read off the picture instead of guessed at, \
                                    rendered, and guessed again. A position is an offset \
                                    from where the layer already rests, so what the ruler \
                                    gives you is the distance to move it. Default false, \
                                    because the lines are drawn onto the frame itself — \
                                    including a PNG kept with `out` — so ask for them while \
                                    measuring and leave them off for a picture to keep."
                },
                "out": {
                    "type": "string",
                    "description": "Also keep the PNG at this path, e.g. review/title.png. \
                                    One instant only — a path names a file, and several \
                                    frames do not fit in one, so asking for a list and a \
                                    path together is refused; `scorsese render --stills` \
                                    is how a set of PNGs gets written. Without it the \
                                    picture is returned and nothing is left on disk."
                }
            },
            "required": ["project", "at"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let instants = instants(arguments, project.timeline_fps)?;
        let kept = kept(arguments, instants.len())?;
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
        let renderer = Renderer::new(&tools, settings);

        let ruled = arguments
            .get("grid")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let mut parts = Vec::with_capacity(instants.len());
        for at in instants {
            let mut frame = renderer
                .still(&project, &dir, at)
                .map_err(|error| format!("compositing frame {}: {error}", at.get()))?;
            // After compositing, over the finished frame: the ruler is
            // furniture for reading the picture, never a layer of the edit.
            if ruled {
                grid::draw(&mut frame);
            }

            // Written to a file either way: PNG encoding is ffmpeg's, and
            // ffmpeg writes files. Where it goes is the only difference — a
            // path the caller named, kept, or a scratch file that is read back
            // and removed.
            let png = Scratch::at(kept);
            frames::write_png(&tools, &png.path, &frame)
                .map_err(|error| format!("writing the frame: {error}"))?;
            let bytes = std::fs::read(&png.path)
                .map_err(|error| format!("reading {} back: {error}", png.path.display()))?;

            let seconds = project.timeline_fps.seconds(at);
            let ruler = if ruled { ", ruled 0.0 to 1.0" } else { "" };
            let mut said = format!(
                "frame {} ({seconds:.2}s) of {} at {resolution}{ruler}",
                at.get(),
                project.name
            );
            if let Some(path) = kept {
                said.push_str(&format!(" — written to {path}"));
            }
            parts.push(Part::picture(said, &bytes));
        }
        Ok(parts.into())
    }
}

/// What the `at` argument says when it says nothing usable.
const WANTED: &str = "`at` is required: a time like 9.1s, a frame like 285, or a list of either";

/// Which timeline frames the `at` argument names, in the order it named them.
///
/// Order is the caller's and is never sorted or deduplicated, unlike
/// `render --stills`. A list of instants is a list of questions, and the
/// answers have to line up with them — a client that asked about the end and
/// then the start reads the reply in that order.
fn instants(arguments: &Value, fps: Fps) -> Result<Vec<Frames>, String> {
    let asked = match arguments.get("at") {
        Some(Value::String(one)) => vec![one.as_str()],
        Some(Value::Array(many)) => many
            .iter()
            .map(|item| {
                item.as_str().ok_or_else(|| {
                    format!(
                        "at: {item} is not an instant — each one is text, like \"9.1s\" or \"285\""
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(WANTED.to_owned()),
    };
    if asked.is_empty() {
        return Err("at: an empty list names no instant to look at".to_owned());
    }
    asked
        .into_iter()
        .map(|at| {
            at.parse::<Cue>()
                .map(|cue| cue.timeline_frame(fps))
                .map_err(|problem| format!("at: {problem}"))
        })
        .collect()
}

/// The path a PNG is kept at, once it is established that one frame was asked
/// for.
///
/// Several instants and a path together is refused rather than reinterpreted
/// as a directory. `out` is the secondary use of this tool — the picture in
/// the reply is the point of it — and `scorsese render --stills` already
/// writes a numbered set of PNGs, so a second, worse version of that here
/// would be a rule to remember instead of a capability.
fn kept(arguments: &Value, instants: usize) -> Result<Option<&str>, String> {
    let out = arguments.get("out").and_then(Value::as_str);
    match out {
        Some(path) if instants > 1 => Err(format!(
            "out: {path} is one path and {instants} instants were asked for. Ask for one \
             instant to keep a file, or use `scorsese render --stills` for a set of PNGs."
        )),
        _ => Ok(out),
    }
}
