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
//!
//! With `sheet: true` the instants come back as **one** picture instead, tiled
//! and labelled the way `look` tiles a file ([`sheet`], #814): five small cells
//! cost less than one full frame, and the change between them is visible
//! because they sit side by side.

mod raster;
mod sheet;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_core::{Fps, Frames};
use scorsese_render::{Cue, RenderSettings, Renderer, Tools, frames, grid};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::scratch::Scratch;
use crate::tools::{Costs, Part, Reply, Tool};

/// Frames, composited and handed back as pictures.
pub(crate) struct Still;

/// What `still` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Which instant to look at: a time like 9.1s, or a timeline frame number
    /// like 285. A bare decimal is refused — say which unit you mean. Give a
    /// list, e.g. ["0s", "9.1s", "400"], to look at several at once: one
    /// sentence and one picture comes back per instant, in the order asked.
    #[schemars(schema_with = "at_schema")]
    at: Value,
    /// The shape to look in, e.g. "9:16" for a vertical edit or "1:1" for a
    /// square one: a preview of the default's pixels in that shape (720x1280
    /// for 9:16). Or an exact raster, e.g. 1080x1920. Default: the default's
    /// pixels in the shape of the edit's first sized picture (the first shot,
    /// photo or generated clip on the lowest video track), 1280x720 when
    /// nothing on it has a size — the reply names the clip it took the shape
    /// from. Layout is a fraction of the frame, so a preview is the same
    /// picture as the delivery and a smaller reply; there is no need to ask
    /// for the delivery raster to see a layout. The exception is a clip with
    /// fit: native, which is a fixed count of pixels and so looks bigger in a
    /// smaller frame than it will in the delivery — ask for the delivery
    /// raster to judge the size of one. With sheet: true it is each cell's
    /// raster instead, a quarter of the pixels (640x360, 360x640 for 9:16).
    resolution: Option<String>,
    /// Rule the picture with coordinates: a line every 0.1 of the frame,
    /// heavier at 0.5, labelled along the top and left edges, origin at the
    /// top-left corner. Fractions of the raster, which is the unit
    /// transform.position.x and transform.position.y are written in — so where
    /// a layer sits is read off the picture instead of guessed at, rendered,
    /// and guessed again. A position is an offset from where the layer already
    /// rests, so what the ruler gives you is the distance to move it. Default
    /// false, because the lines are drawn onto the frame itself — including a
    /// PNG kept with `out` — so ask for them while measuring and leave them off
    /// for a picture to keep.
    #[serde(default)]
    grid: bool,
    /// Answer with one picture instead of one per instant: the frames tiled
    /// into a contact sheet, at most 5, each labelled with its time and
    /// timeline frame — the shape look gives a video file. Cheaper than
    /// separate pictures and better for comparing them, since what changed
    /// between two instants is only visible side by side. With grid: true each
    /// cell is ruled on its own. With out, the sheet is the one file kept.
    /// Default false.
    #[serde(default)]
    sheet: bool,
    /// Also keep the PNG at this path, e.g. review/title.png. One instant only,
    /// or sheet: true — a path names a file, and several separate frames do not
    /// fit in one, so asking for a list and a path together without a sheet is
    /// refused; `scorsese render --stills`
    /// is how a set of PNGs gets written. Without it the picture is returned
    /// and nothing is left on disk. A relative path is relative to the project
    /// directory, never the server's working directory; an absolute one is
    /// used as given.
    out: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("at", WANTED)];
}

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
         one call rather than one per section. Pictures come back preview-sized \
         in the shape of the edit's first sized clip; for a vertical edit whose \
         timeline does not say so (titles, pages, landscape footage cropped \
         upright) pass resolution: \"9:16\" — never the delivery size, which \
         costs more and shows the same layout. Needs ffmpeg, but encodes \
         nothing: seconds, not a whole render. Use it to check what \
         project_describe can only assert — that a title is readable, that a \
         layer is where it was meant to be, that a cut lands. Sketch and stale \
         generated assets appear as slug cards, so a frame of an unrealised \
         shot still shows something. A web page on screen is captured first \
         (once; later calls reuse it), and a note under the frame says what it \
         could not load or why it could not be captured. Pass grid: true to have the frame ruled in \
         the fractions the document itself takes, so a coordinate is read off \
         the picture rather than converged on by guessing. Pass sheet: true to \
         get up to 5 instants back as one labelled contact sheet instead of \
         one picture each — cheaper, and the frames can be compared side by \
         side."
    }

    fn costs(&self) -> Costs {
        Costs::Frames
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let project = load(dir)?;
        let instants = instants(&arguments.at, project.timeline_fps)?;
        // A sheet is one picture however many instants are in it, so it is one
        // file as far as `out` is concerned.
        let files = if arguments.sheet { 1 } else { instants.len() };
        let kept = kept(dir, arguments.out.as_deref(), files)?;
        let budget = if arguments.sheet {
            raster::CELL
        } else {
            raster::FRAME
        };
        let (resolution, shaped_by) =
            raster::choose(arguments.resolution.as_deref(), &project, budget)?;
        // Said once, in the first sentence: a preview whose shape was guessed
        // from a clip names it, so a wrong guess is one argument to correct.
        let shaped = shaped_by
            .map(|asset| format!(" (the shape of {asset}; pass resolution: \"9:16\" or another aspect for a different one)"))
            .unwrap_or_default();

        // Discovered per call rather than held, as `render` does: a server that
        // found ffmpeg at startup would keep insisting it was there after
        // someone uninstalled it.
        let tools = Tools::discover().map_err(|error| format!("{error}"))?;
        // The project's own grid, so the frame handed back is the frame asked
        // for rather than the nearest one at some other rate.
        let settings = RenderSettings::new(resolution, project.timeline_fps);
        let renderer = Renderer::new(&tools, settings);

        let ruled = arguments.grid;
        if arguments.sheet {
            return sheet::reply(sheet::Asked {
                tools: &tools,
                renderer: &renderer,
                project: &project,
                dir,
                instants: &instants,
                resolution,
                shaped: &shaped,
                ruled,
                kept: kept.as_ref().map(|(given, path)| (*given, path.as_path())),
            });
        }

        let mut parts = Vec::with_capacity(instants.len());
        let mut told = Vec::new();
        for (index, at) in instants.into_iter().enumerate() {
            let (mut frame, notes) = renderer
                .still_noted(&project, dir, at)
                .map_err(|error| format!("compositing frame {}: {error}", at.get()))?;
            // After compositing, over the finished frame: the ruler is
            // furniture for reading the picture, never a layer of the edit.
            if ruled {
                grid::draw(&mut frame);
            }

            let bytes = png(
                &tools,
                kept.as_ref().map(|(_, path)| path.as_path()),
                &frame,
            )?;

            let seconds = project.timeline_fps.seconds(at);
            let ruler = if ruled { ", ruled 0.0 to 1.0" } else { "" };
            let mut said = format!(
                "frame {} ({seconds:.2}s) of {} at {resolution}{}{ruler}",
                at.get(),
                project.name,
                if index == 0 { shaped.as_str() } else { "" },
            );
            if let Some((given, _)) = &kept {
                said.push_str(&format!(" — written to {given}"));
            }
            // Under the frame it was noticed drawing, and once a call: a page
            // on screen at several instants is one page with one problem.
            for note in notes {
                if !told.contains(&note) {
                    said.push_str(&format!("\nnote: {note}"));
                    told.push(note);
                }
            }
            parts.push(Part::picture(said, &bytes));
        }
        Ok(parts.into())
    }
}

/// Encodes `frame` as PNG bytes, kept at `path` if one was named.
///
/// Written to a file either way: PNG encoding is ffmpeg's, and ffmpeg writes
/// files. Where it goes is the only difference — a path the caller named, kept,
/// or a scratch file that is read back and removed.
fn png(
    tools: &Tools,
    path: Option<&std::path::Path>,
    frame: &scorsese_render::Frame,
) -> Result<Vec<u8>, String> {
    let png = Scratch::at(path);
    frames::write_png(tools, &png.path, frame)
        .map_err(|error| format!("writing the frame: {error}"))?;
    std::fs::read(&png.path)
        .map_err(|error| format!("reading {} back: {error}", png.path.display()))
}

/// What the `at` argument is, for the refusal when it says nothing usable.
const WANTED: &str = "a time like 9.1s, a frame like 285, or a list of either";

/// The shape `at` takes on the wire: one instant as text, or a list of them.
///
/// Read as a bare JSON value and taken apart by [`instants`] rather than by
/// serde, because the refusals it gives — which item in a list is not text,
/// that a list is empty — say more than a type error could.
pub(super) fn at_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "type": ["string", "array"],
        "items": { "type": "string" }
    })
}

/// Which timeline frames the `at` argument names, in the order it named them.
///
/// Order is the caller's and is never sorted or deduplicated, unlike
/// `render --stills`. A list of instants is a list of questions, and the
/// answers have to line up with them — a client that asked about the end and
/// then the start reads the reply in that order.
///
/// Shared with `project_describe`, which takes the same argument for the same
/// reason: `at` has to mean one thing across the whole server, so a client
/// that learned `["0s", "9.1s"]` here can write it there.
pub(super) fn instants(at: &Value, fps: Fps) -> Result<Vec<Frames>, String> {
    let asked = match at {
        Value::String(one) => vec![one.as_str()],
        Value::Array(many) => many
            .iter()
            .map(|item| {
                item.as_str().ok_or_else(|| {
                    format!(
                        "at: {item} is not an instant — each one is text, like \"9.1s\" or \"285\""
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(format!("`at` is required: {WANTED}")),
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
///
/// Comes back as the caller's own words and the file they resolve to: a
/// relative path is the project's, not the server's working directory (#518),
/// and the reply says back the string it was given because that is the one a
/// later call resolves the same way.
fn kept<'a>(
    dir: &std::path::Path,
    out: Option<&'a str>,
    instants: usize,
) -> Result<Option<(&'a str, std::path::PathBuf)>, String> {
    let Some(given) = out else {
        return Ok(None);
    };
    if instants > 1 {
        return Err(format!(
            "out: {given} is one path and {instants} instants were asked for. Ask for one \
             instant to keep a file, pass sheet: true to keep them as one contact sheet, or \
             use `scorsese render --stills` for a set of PNGs."
        ));
    }
    Ok(args::under(dir, out, "out")?.map(|path| (given, path)))
}
