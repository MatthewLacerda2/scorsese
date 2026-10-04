//! The tools, and the one rule about them.
//!
//! **Every tool carries a description, and every argument carries one too.**
//! A description is not a courtesy: it is the entire interface a client has to
//! a tool. An undescribed tool is a capability that exists and cannot be
//! found — nothing fails, the assistant on the other end simply never calls
//! it. `tests/described.rs` walks this registry and fails on one that is
//! missing, from the first tool onwards.
//!
//! Nothing here holds session state. Every tool takes the project directory it
//! works on, reads it, does one thing, and returns. That makes each call
//! independent of every other, which is what lets a client crash, reconnect,
//! or run two conversations against one project without a server-side notion
//! of "the open project" going stale behind its back.

mod args;
mod authoring;
mod confirm;
mod create;
mod design;
mod edit;
mod generate;
mod hear;
mod icons;
mod inspect;
mod jobs;
mod level;
mod look;
mod scratch;
mod script;
mod still;
mod synth;
mod voices;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use scorsese_render::Cancel;
use serde_json::Value;

use crate::renders::{Renders, Report};

/// The `mimeType` an image block carries. One kind, because there is one kind
/// of picture this server produces — see [`scorsese_render::frames`] for why it
/// is PNG and not something smaller.
const PNG: &str = "image/png";

/// One thing an answer is about: what to say, and the picture it names when
/// there is one.
///
/// Text, always: an answer a client cannot show as words is an answer nobody
/// can read back in a transcript. A picture *as well*, when the answer is one —
/// MCP carries an image as its own content block, so a client that can see
/// images sees the frame rather than a path to a file it has no way to open.
///
/// The two are not alternatives, and that is the point of the shape. A tool
/// that returned only an image would be unreadable to a client without vision
/// and unloggable everywhere; one that returned only text could never show a
/// frame at all.
pub struct Part {
    /// What to say. The whole answer for every tool that has nothing to show.
    pub text: String,
    /// A PNG, already base64-encoded, when there is a picture in this part.
    pub image: Option<String>,
}

impl Part {
    /// Words, with nothing to show. What a reply of several parts is built
    /// from when none of them is a picture — a whole document and the note
    /// that goes with it, kept in separate blocks so the document arrives
    /// exactly as it is on disk.
    pub(crate) fn words(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            image: None,
        }
    }

    /// A picture, and the words that go with it.
    pub(crate) fn picture(text: String, png: &[u8]) -> Self {
        Self {
            text,
            image: Some(STANDARD.encode(png)),
        }
    }
}

/// What a tool answers with: one part, or several in the order to read them.
///
/// Several, because a question is often asked about several things at once —
/// `still` handed six instants answers about all six — and the alternative is
/// a client paying for six round trips to ask one question. What the list
/// preserves is which words go with which picture: a block of prose about six
/// frames followed by six frames is unreadable, so each sentence sits directly
/// above the picture it names.
pub struct Reply {
    /// What the answer is made of, in the order a client renders it. Never
    /// empty — a reply that says nothing is not an answer.
    pub parts: Vec<Part>,
}

impl Reply {
    /// The content blocks MCP puts this on the wire as.
    ///
    /// Text first, always. A client renders blocks in order, and the sentence
    /// saying which frame this is belongs above the frame rather than under it.
    pub fn content(&self) -> Vec<Value> {
        let mut blocks = Vec::with_capacity(self.parts.len() * 2);
        for part in &self.parts {
            blocks.push(serde_json::json!({ "type": "text", "text": part.text }));
            if let Some(image) = &part.image {
                blocks.push(serde_json::json!({
                    "type": "image",
                    "data": image,
                    "mimeType": PNG
                }));
            }
        }
        blocks
    }
}

impl From<Vec<Part>> for Reply {
    fn from(parts: Vec<Part>) -> Self {
        Self { parts }
    }
}

impl From<String> for Reply {
    fn from(text: String) -> Self {
        Self {
            parts: vec![Part { text, image: None }],
        }
    }
}

impl From<&str> for Reply {
    fn from(text: &str) -> Self {
        Self::from(text.to_owned())
    }
}

/// What a call spends, beyond the document it is handed.
///
/// A small closed set rather than a string. "Does this cost money, a process,
/// or minutes?" is a question with a handful of honest answers here, and a
/// free-form string would drift into prose the first time anyone had a nuance
/// to add — at which point it stops being something a client can compare.
///
/// It lives on the tool for the same reason [`Tool::description`] does: it is a
/// fact about the tool, so it should be answerable where the tool is edited
/// rather than in a page whose author may never open this file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Costs {
    /// The document, and arithmetic over it. No process, no network, no money.
    Nothing,
    /// One `ffprobe` per file measured — reading a header, not decoding.
    Probe,
    /// `ffmpeg` decodes, and that is the whole of it: no encoder runs.
    Decode,
    /// `ffmpeg` decodes and the compositor draws, once per frame asked for —
    /// so seconds, and they scale with how many instants were named.
    Frames,
    /// `ffmpeg` encodes. Roughly the running time of whatever is rendered,
    /// which makes this the one cost here a person waits on.
    Encode,
    /// A call to a provider that is **not billed** — a listing, a lookup.
    ///
    /// Its own answer rather than [`Costs::Nothing`], because that one promises
    /// no network at all and a client may reasonably act on the promise: a tool
    /// needing a key and a connection is not one to reach for while offline,
    /// however little it spends. Equally not [`Costs::Money`], which is the one
    /// cost here that cannot be undone by trying again.
    Request,
    /// **Real money, to somebody else.** The only cost on this list that is not
    /// paid in seconds, and the only one that cannot be undone by waiting — so
    /// it is its own answer rather than a note on `Encode`, and a client
    /// choosing between tools should be able to see it without reading prose.
    Money,
}

impl Costs {
    /// How the cost reads in a table cell, and to anyone being shown it.
    pub fn says(self) -> &'static str {
        match self {
            Self::Nothing => "nothing",
            Self::Probe => "ffprobe",
            Self::Decode => "ffmpeg",
            Self::Frames => "ffmpeg, and seconds",
            Self::Encode => "ffmpeg, and real time",
            Self::Request => "a key and a network, but no money",
            Self::Money => "money, at a provider",
        }
    }
}

/// What a tool needs to say about itself, and what it does.
pub trait Tool: Send + Sync {
    /// How a client names it. Stable — renaming one breaks every saved prompt
    /// that mentions it.
    fn name(&self) -> &'static str;

    /// What it does, in the words a client shows to whoever is deciding
    /// whether to call it.
    ///
    /// **The first sentence stands alone.** It is the tool's one-line identity
    /// — the cell `docs/mcp.md` carries is generated from exactly this string —
    /// so it has to say what the tool is for without the rest of the paragraph
    /// standing behind it.
    fn description(&self) -> &'static str;

    /// What calling it spends. Required, with no default: the tool that costs
    /// real time and does not say so is precisely the one worth knowing about,
    /// and a default is how it would stay quiet.
    fn costs(&self) -> Costs;

    /// The JSON Schema of its arguments. Every property carries its own
    /// `description`, for the same reason the tool does.
    fn schema(&self) -> Value;

    /// Runs it. The `Ok` reply is what the client sees; the `Err` string is
    /// what it sees when the tool refused, which is just as much of an answer.
    ///
    /// A [`Reply`] is a `String` away — `Ok(text.into())` — so a tool with
    /// nothing to show says so in one word rather than in a struct literal.
    fn call(&self, arguments: &Value) -> Result<Reply, String>;

    /// Runs it under `cancel`, which the client trips by cancelling the
    /// request (`notifications/cancelled`).
    ///
    /// Only a tool that runs long enough to be worth stopping overrides this —
    /// `render` (#647) and `synth_bake` (#661). Every other one finishes in
    /// the time it takes to notice, so it runs to the end and its answer is
    /// simply not sent.
    fn call_cancellable(&self, arguments: &Value, cancel: &Cancel) -> Result<Reply, String> {
        let _ = cancel;
        self.call(arguments)
    }

    /// Runs it inside a stdio session, which is how `serve` runs every call:
    /// under the call's cancel, beside the session's renders, with somewhere
    /// to report progress when the client asked for it.
    ///
    /// Only `render`, `jobs` and `job_cancel` (#700) override this — they are
    /// the tools with something to say about work that outlives a call. Every
    /// other one is [`Tool::call_cancellable`].
    fn call_in(&self, arguments: &Value, context: &mut Context<'_>) -> Result<Reply, String> {
        self.call_cancellable(arguments, context.cancel)
    }
}

/// What a call runs with inside a stdio session, besides its arguments.
///
/// Published only because [`Tool::call_in`] names it; nothing outside this
/// crate makes one. The hosted server runs tools through
/// [`Tool::call_cancellable`] and has a job queue of its own.
pub struct Context<'a> {
    /// Tripped when the client cancels the call.
    cancel: &'a Cancel,
    /// The renders the session has started.
    renders: &'a Renders,
    /// Where `notifications/progress` go, when the call asked for them.
    report: Option<Report<'a>>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        cancel: &'a Cancel,
        renders: &'a Renders,
        report: Option<Report<'a>>,
    ) -> Self {
        Self {
            cancel,
            renders,
            report,
        }
    }
}

/// Every tool this server exposes.
pub fn registry() -> Vec<Box<dyn Tool>> {
    vec![
        // First, because it is the first call on a machine with no project on
        // it and a client reads this list in order.
        Box::new(create::New),
        Box::new(inspect::Read),
        Box::new(inspect::Describe),
        Box::new(inspect::Check),
        Box::new(inspect::Assets),
        Box::new(edit::Import),
        Box::new(edit::Probe),
        Box::new(script::Read),
        Box::new(script::Write),
        Box::new(edit::Write),
        // Before place_clip, because they are how there comes to be anything
        // to place and anywhere to put it: a lane, and the four kinds of asset
        // that no import and no provider brings in.
        Box::new(authoring::TrackNew),
        Box::new(authoring::TextNew),
        Box::new(authoring::ColorNew),
        Box::new(authoring::ShapeNew),
        Box::new(authoring::IconNew),
        Box::new(authoring::AssetSet),
        Box::new(edit::Sequence),
        Box::new(authoring::AssetRemove),
        Box::new(authoring::TrackRemove),
        // Before the tools that decorate a cut, because they are how there
        // comes to be one: a clip has to be on the timeline before anything
        // can dissolve it or scale it.
        Box::new(edit::PlaceClip),
        Box::new(edit::TrimClip),
        Box::new(edit::ClipSet),
        Box::new(edit::ClipAnimate),
        Box::new(edit::ClipFollow),
        Box::new(edit::ClipMove),
        Box::new(edit::ClipRemove),
        Box::new(edit::ClipGroup),
        Box::new(edit::ClipUngroup),
        Box::new(edit::Dissolve),
        Box::new(edit::Duck),
        Box::new(edit::SetVolume),
        Box::new(edit::ScalePacing),
        Box::new(synth::New),
        // Beside the tool that starts a recipe from one: choosing an
        // instrument and starting a sound are one thought.
        Box::new(synth::Kit),
        Box::new(synth::Import),
        Box::new(synth::Export),
        Box::new(synth::Read),
        Box::new(synth::Write),
        Box::new(synth::Set),
        Box::new(synth::Check),
        Box::new(synth::Bake),
        Box::new(synth::Survey),
        Box::new(level::Level),
        // Beside the tools that read rather than the ones that write: it
        // answers a question about this build, not about the project, and it is
        // what a client calls before writing an icon asset at all.
        Box::new(icons::Icons),
        Box::new(voices::Voices),
        Box::new(design::VoiceDesign),
        // Beside the tool it feeds rather than with the other document verbs:
        // editing a brief and realising it are one thought, and a client
        // reading this list in order should meet them together.
        Box::new(edit::Rebrief),
        Box::new(generate::Generate),
        Box::new(edit::Render),
        // Beside the tool that starts what they follow: a render runs in the
        // background, and these are how anyone learns how it went.
        Box::new(jobs::Jobs),
        Box::new(jobs::JobCancel),
        Box::new(still::Still),
        Box::new(look::Look),
        Box::new(hear::Hear),
    ]
}

pub(crate) fn find(name: &str) -> Option<Box<dyn Tool>> {
    registry().into_iter().find(|tool| tool.name() == name)
}

/// The project directory an argument object names.
///
/// Every tool takes one, and it is required rather than defaulted to the
/// working directory: a server started by a client has no meaningful working
/// directory, and guessing one is how you edit the wrong film.
pub(crate) fn project_dir(arguments: &Value) -> Result<std::path::PathBuf, String> {
    arguments
        .get("project")
        .and_then(Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "`project` is required: the path of the *.scor directory".to_owned())
}

/// One path argument, resolved against the project directory unless it is
/// already absolute.
///
/// Relative-to-the-project is the rule every path in this surface obeys, for
/// the reason [`project_dir`] gives: the server's working directory belongs to
/// whoever launched it, so a relative path resolved against it lands somewhere
/// the caller did not name and cannot read back (#496). An absolute path is
/// still honoured, because a measured render or a partial bake is as likely to
/// sit outside the project as in it. Whether a *write* may leave the project is
/// a separate question this does not answer.
pub(crate) fn under(
    dir: &std::path::Path,
    arguments: &Value,
    field: &str,
) -> Result<Option<std::path::PathBuf>, String> {
    let Some(given) = arguments.get(field).and_then(Value::as_str) else {
        return Ok(None);
    };
    if given.trim().is_empty() {
        return Err(format!("`{field}` is empty — give a path or leave it out"));
    }
    let path = std::path::PathBuf::from(given);
    Ok(Some(if path.is_absolute() {
        path
    } else {
        dir.join(path)
    }))
}

/// The `project` property, spelled the same way in every tool's schema.
///
/// `project_new` writes its own, and only its own: the directory it names is
/// one to make rather than one to work on. The *name* of the argument is what
/// has to be shared, and it is.
pub(crate) fn project_property() -> Value {
    serde_json::json!({
        "type": "string",
        "description": "Path to the *.scor project directory to work on."
    })
}
