//! Footage, as a picture the client can actually look at.
//!
//! The other half of [`still`](super::still). That one shows the **edit** — the
//! timeline composited at an instant. This shows the **material**: frames out
//! of a video file — an imported asset, or footage nobody has imported yet.
//!
//! It is the largest gap between what this server promises and what it could
//! do. Every other tool answers about a file in words — its duration, its
//! shape, its codec, its name — and words are not what a shot is. An assistant
//! handed twenty clips and asked for a cut is arranging file names and hoping;
//! with this it can find out that clip 3 is a wide establishing shot and clip 7
//! is the same subject in close-up.
//!
//! **One picture, not five.** Five separate image blocks cost five times as
//! much and say less, because what tells you what a shot *does* is the change
//! between frames, and a change is only visible when they are side by side.
//!
//! **Asked for nothing in particular, it glances** ([`contact::glance`],
//! #1563): frames chosen across the whole file, with blank ones and
//! near-repeats left out, and the transcript beside the file in the reply's
//! words when there is one. Named instants — `from`, `to` or `frames` — walk
//! or spread as they always have.

use schemars::JsonSchema;
use scorsese_core::words::Words;
use scorsese_render::contact::{self, Look as Range, MAX_FRAMES, label};
use scorsese_render::{Overlay, RenderAsk, Spelled, Tools};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::scratch::Scratch;
use crate::tools::{Costs, Part, Reply, Tool};

/// Frames of a file, tiled into one sheet.
pub(crate) struct Look;

/// What `look` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The video to look at, as a path relative to the project — e.g.
    /// assets/03-rooftop.mp4 — or an absolute path to footage that has not
    /// been imported. Not an asset id: this reads a file, and the file need not
    /// be in the assets table at all.
    file: String,
    /// Where to start, in seconds. To walk a long file, pass the `from` the
    /// previous reply named. Without `from`, `to` and `frames` the whole file
    /// is glanced at instead.
    from: Option<f64>,
    /// Where to stop, in seconds. Without it the frames are 5 seconds apart,
    /// which is how a file gets covered; with it they spread evenly across the
    /// span you named, which is how one stretch gets looked at closely.
    to: Option<f64>,
    /// How many frames to take, at most 5: more than that in one sheet a
    /// caller walks is unreadable at any size worth sending.
    frames: Option<u64>,
    /// Rule every frame of the sheet with coordinates: a line every 0.1,
    /// heavier at 0.5, labelled along the top and left edges, origin at the
    /// top-left corner. The fractions are the source's own — its whole width
    /// and height, not the render raster and not the sheet — which is exactly
    /// what a clip's crop is measured in, so the rectangle read off a frame is
    /// the rectangle written into the document. Default false, because the
    /// lines are drawn onto the picture itself.
    #[serde(default)]
    grid: bool,
    /// A platform id (youtube_shorts, instagram_reels, instagram_reels_ad,
    /// instagram_stories_ad, tiktok, tiktok_ad) whose feed interface is drawn
    /// over every frame, to judge a 9:16 file — a render, say — as the
    /// platform shows it. The controls are an approximation scorsese draws.
    overlay: Option<String>,
    /// true tints what the overlay's interface covers, from the vendor's own
    /// published figures where there are any. Needs overlay.
    #[serde(default)]
    safe_zones: bool,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("file", "the path of a video to look at")];
}

impl Tool for Look {
    fn name(&self) -> &'static str {
        "look"
    }

    fn description(&self) -> &'static str {
        "Look at the footage itself, not the edit. Frames sampled from a video \
         file and tiled into one contact sheet, each labelled with the moment it \
         came from. Works on any video the \
         project can reach — an imported asset, or a file on disk that is not \
         in the project yet — and answers what no amount of metadata can: what \
         is actually in the shot. Use it before cutting unfamiliar footage, and \
         after generating a shot, to see whether what came back is what the \
         brief asked for. With only `file`, a glance at the whole video: 3 to \
         12 frames, more for a longer file, chosen evenly with blank frames and \
         near-repeats left out, and the file's transcript in the reply when it \
         has one (this never starts a transcription; `transcribe` does). Give \
         `from`, `frames` or `to` to choose the moments instead: at most 5 \
         frames, one every 5 seconds from `from`; a longer file is walked with \
         successive calls, and each reply says where the next one starts. Give \
         `to` to look closely at one stretch — the frames then spread evenly \
         across it. Pass grid: true to have \
         every frame ruled in fractions of the source, which is what a clip's \
         crop is measured in. Pass overlay: tiktok (any 9:16 platform) to see \
         each frame under that feed's interface, and safe_zones: true to tint the \
         vendor's covered margins; the frames are then a walk, never a glance. \
         Needs ffmpeg; decodes at most two dozen frames (a glance) or five, and \
         encodes nothing."
    }

    fn writes(&self) -> bool {
        false
    }

    fn costs(&self) -> Costs {
        Costs::Decode
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        // Relative to the project, like every other path in this system;
        // absolute when the footage is somewhere else entirely, which is the
        // case for material nobody has imported yet.
        let file = arguments.project.dir().join(&arguments.file);
        let range = range(&arguments)?;
        // An overlay is drawn over a walk's frames, never a glance's.
        let glancing = arguments.from.is_none()
            && arguments.to.is_none()
            && arguments.frames.is_none()
            && range.overlay.is_none();

        // Discovered per call rather than held, as the other ffmpeg tools do: a
        // server that found ffmpeg at startup would keep insisting it was there
        // after somebody uninstalled it.
        let tools = Tools::discover().map_err(|error| format!("{error}"))?;
        if glancing {
            let words = Words::heard_in(&file);
            let glance = contact::glance(&tools, &file, words.as_ref(), range.grid)
                .map_err(|error| format!("{error}"))?;
            let bytes = Scratch::png(&tools, &glance.sheet.image, None, "the sheet")?;
            return Ok(vec![Part::picture(glanced(&glance, range.grid), &bytes)].into());
        }
        let sheet = contact::sheet(&tools, &file, &range).map_err(|error| format!("{error}"))?;

        // Through a file, because PNG encoding is ffmpeg's and ffmpeg writes
        // files. Nothing is left behind: a sheet is a thing to look at, not an
        // artifact of the project, and `scorsese look` is how one gets kept.
        let bytes = Scratch::png(&tools, &sheet.image, None, "the sheet")?;

        let mut words = said(&sheet, range.grid);
        if let Some(overlay) = range.overlay {
            words.push_str(&format!(" — over each frame: {}", overlay.said()));
        }
        Ok(vec![Part::picture(words, &bytes)].into())
    }
}

/// What the reply says in words.
///
/// The moments are named as well as drawn, because a client without vision has
/// to get a useful answer too — the rule the other picture tools already
/// follow. Where to carry on from is said outright rather than left to be
/// worked out: walking a long file is the expected use, and a step somebody
/// computes is a step somebody gets wrong.
fn said(sheet: &contact::Sheet, ruled: bool) -> String {
    let moments: Vec<String> = sheet.at_seconds.iter().map(|at| label(*at)).collect();
    let mut text = format!(
        "{} of {} ({:.1}s long), at {}{}",
        contact::counted(moments.len()),
        sheet.file.display(),
        sheet.duration_seconds,
        moments.join(", "),
        if ruled {
            " — each ruled 0.0 to 1.0 of the source"
        } else {
            ""
        }
    );
    match sheet.next_from() {
        Some(next) => text.push_str(&format!(
            " — {next:.1}s to {:.1}s not seen yet; call again with from: {next}",
            sheet.duration_seconds
        )),
        None => text.push_str(" — that is the whole file"),
    }
    text
}

/// What a glance's reply says in words: the moments, as [`said`] names
/// them, and the transcript or that there is none.
fn glanced(glance: &contact::Glance, ruled: bool) -> String {
    let sheet = &glance.sheet;
    let moments: Vec<String> = sheet.at_seconds.iter().map(|at| label(*at)).collect();
    let mut text = format!(
        "{} across the whole of {} ({:.1}s long), at {}, blank frames and near-repeats \
         left out{}",
        contact::counted(moments.len()),
        sheet.file.display(),
        sheet.duration_seconds,
        moments.join(", "),
        if ruled {
            " — each ruled 0.0 to 1.0 of the source"
        } else {
            ""
        }
    );
    match &glance.transcript {
        Some(lines) => text.push_str(&format!(
            ". What is said, by when it starts, in the frames' times:\n{lines}"
        )),
        None => text.push_str(". It has no transcript."),
    }
    text
}

/// What part of the file the arguments name.
///
/// The cap is enforced in the library below this, not here, so it holds for the
/// CLI as well — but a caller who asks for fifty is told rather than silently
/// given five. A limit that quietly rewrites the request teaches nobody
/// anything.
fn range(arguments: &Arguments) -> Result<Range, String> {
    let frames = match arguments.frames {
        None => MAX_FRAMES,
        Some(asked) if asked as usize > MAX_FRAMES => {
            return Err(format!(
                "frames: {asked} is more than the {MAX_FRAMES} a sheet of chosen moments \
                 holds. Ask for {MAX_FRAMES}, then call again from where the reply says — \
                 or leave out from, to and frames for a glance across the whole file."
            ));
        }
        Some(asked) => asked as usize,
    };
    let from = arguments.from.unwrap_or(0.0);
    let to = arguments.to;
    if to.is_some_and(|to| to < from) {
        return Err(format!("to: {to:?} is before from: {from}"));
    }
    Ok(Range {
        from_seconds: from,
        to_seconds: to,
        count: frames,
        grid: arguments.grid,
        overlay: overlay(arguments)?,
    })
}

/// The overlay the arguments name, read by the rules a render reads it by.
fn overlay(arguments: &Arguments) -> Result<Option<Overlay>, String> {
    let ask = RenderAsk::read(&Spelled {
        overlay: arguments.overlay.as_deref(),
        safe_zones: Some(arguments.safe_zones),
        ..Spelled::default()
    });
    ask.map(|ask| ask.overlay)
        .map_err(|problem| problem.to_string())
}
