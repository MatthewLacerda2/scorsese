//! Web pages: writing one, and reading it back.
//!
//! The pair `synth_read`/`synth_write` and `script_read`/`script_write` already
//! are, for the third authored document. A page is HTML and CSS — the
//! vocabulary a model writes motion graphics in best (#594) — so the whole of
//! authoring one is handing over its text. There is deliberately no
//! `page_new`: writing a name nothing answers to makes the page, the way
//! `sequence` makes the sequence it is asked to change, and every name on the
//! tool list is paid for on every model call (#780).
//!
//! **The files beside the pages go through the same two tools** (#954): a
//! script or stylesheet several pages load by relative path is named with
//! `file` instead of `page`, rather than by a tool of its own — on the web
//! there is no other way to write one, and a third name would be paid for on
//! every call.
//!
//! How a page is written well — the contract it is drawn under, what it can
//! load offline, why seekable animation beats an integrated loop — is
//! `docs/pages.md`, which the descriptions point at as `guide pages` (#909).

use schemars::JsonSchema;
use scorsese_core::{AssetId, AssetKind, read_page_file, write_page, write_page_file};
use serde::Deserialize;
use serde_json::Value;

use super::args::{self, Name, ProjectDir, Required};
use super::inspect::load;
use super::{Costs, Reply, Tool};

/// The `page` argument, described the same way in both tools.
const PAGE: &str = "The page's asset id, e.g. lower-third. project_assets lists the \
                    html assets a project has. Give this or `file`.";

/// The `file` argument, described the same way in both tools.
const FILE: &str = "Instead of `page`: a file beside the pages that pages load by \
                    relative path, e.g. lib.js for <script src=\"lib.js\">, at \
                    pages/<file>. A .js, .css, .json or .svg of at most 1 MB; not an \
                    asset, never placed. Every page that loads it is drawn again \
                    when it changes.";

/// Which of the two a call names: a page by its asset id, or a file beside
/// the pages by its name — exactly one.
fn target(page: Option<Name>, file: Option<Name>) -> Result<Target, String> {
    match (page, file) {
        (Some(page), None) => Ok(Target::Page(page)),
        (None, Some(file)) => Ok(Target::File(file)),
        (None, None) => Err("`page` is required: the page's asset id — or `file` for a \
                             file beside the pages"
            .to_owned()),
        (Some(_), Some(_)) => Err("give `page` or `file`, not both — one call writes one \
                                   file"
            .to_owned()),
    }
}

/// What [`target`] decided.
enum Target {
    Page(Name),
    File(Name),
}

/// Write a page, making it when the name is new.
pub(super) struct Write;

/// What `page_write` takes.
#[derive(Deserialize, JsonSchema)]
struct WriteArguments {
    project: ProjectDir,
    /// The page's asset id, e.g. lower-third. A name no asset has yet makes
    /// an html asset of that id, its file at pages/<id>.html; the id of a page
    /// already there rewrites that page's file. The id of any other kind of
    /// asset is refused. Give this or `file`.
    page: Option<Name>,
    #[schemars(description = FILE)]
    file: Option<Name>,
    /// The complete HTML document — or, with `file`, the file's whole text.
    /// Not a patch: whatever is here replaces the file. `guide pages` has the
    /// contract a page is drawn under.
    html: String,
}

impl args::Arguments for WriteArguments {
    const REQUIRED: Required = &[("html", "the page's HTML, or the file's text")];
}

impl Tool for Write {
    fn name(&self) -> &'static str {
        "page_write"
    }

    fn description(&self) -> &'static str {
        "Write a web page — a title card, a lower third, an animated chart — \
         as an html asset the timeline plays like footage with alpha. A new \
         name makes the asset and its file under pages/; an existing page's \
         name replaces its HTML whole, so read it first with page_read. Then \
         place_clip it on a video track and look at it with still, whose notes \
         say what the page could not load. The page is drawn offline by a \
         headless browser on a clock that only moves with the clip: \
         window.scorsese gives its width, height, fps and duration, and \
         scorsese.clips[id] = {start, end} places every clip beside it in the \
         page's own seconds, so time the page to the narration's clip id \
         rather than copying seconds into it; the shipped \
         fonts are there by name, the motion kit (frame loop on the \
         page's seconds, easings, enter/exit, count-up, seeded random) at \
         https://lib.scorsese/kit.js, anime.js at \
         https://lib.scorsese/anime.min.js and lottie-web at \
         https://lib.scorsese/lottie.min.js, to play a Lottie stock_import \
         wrote beside the pages; where it draws nothing, the tracks \
         below show through. Pages that share helpers load one file beside \
         them by relative path: write it with `file` (lib.js, look.css) \
         instead of `page`. Read `guide pages` before writing one."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<WriteArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: WriteArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let html = &arguments.html;
        let page = match target(arguments.page, arguments.file)? {
            Target::Page(page) => page,
            Target::File(file) => return write_file(dir, file.as_str(), html),
        };
        let mut project = load(dir)?;
        let written = write_page(&mut project, dir, page.as_str(), html)
            .map_err(|error| format!("refused, nothing written — {error}"))?;
        let bytes = html.len();
        if !written.created {
            return Ok(format!(
                "{} rewritten ({bytes} bytes) — page `{}`. Every clip of it shows \
                 the new page at the next still or render.",
                written.path, written.id
            )
            .into());
        }
        project
            .save(dir)
            .map_err(|error| format!("the page was written, but saving the project: {error}"))?;
        Ok(format!(
            "{} — html, new\n{} ({bytes} bytes)\nPlace it with place_clip on a video \
             track, then still to see it.",
            written.id, written.path
        )
        .into())
    }
}

/// `page_write` with `file`: the text lands beside the pages, and the
/// document is not touched — so not even loaded, beyond the folder being a
/// project.
fn write_file(dir: &std::path::Path, name: &str, text: &str) -> Result<Reply, String> {
    load(dir)?;
    let written = write_page_file(dir, name, text)
        .map_err(|error| format!("refused, nothing written — {error}"))?;
    let bytes = text.len();
    let what = if written.created {
        "written"
    } else {
        "rewritten"
    };
    Ok(format!(
        "{} {what} ({bytes} bytes). Every page that loads it is drawn again at \
         the next still or render; a page loads it by relative path, e.g. \
         <script src=\"{name}\">.",
        written.path
    )
    .into())
}

/// A page as it is on disk.
pub(super) struct Read;

/// What `page_read` takes.
#[derive(Deserialize, JsonSchema)]
struct ReadArguments {
    project: ProjectDir,
    #[schemars(description = PAGE)]
    page: Option<Name>,
    #[schemars(description = FILE)]
    file: Option<Name>,
}

impl args::Arguments for ReadArguments {}

impl Tool for Read {
    fn name(&self) -> &'static str {
        "page_read"
    }

    fn description(&self) -> &'static str {
        "Read a web page's HTML exactly as it is on disk — or, with `file`, a \
         file beside the pages such as lib.js. Pair with page_write to change \
         one: read it, change it, write it back, look with still."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<ReadArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: ReadArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let page = match target(arguments.page, arguments.file)? {
            Target::Page(page) => page,
            Target::File(file) => {
                load(dir)?;
                return read_page_file(dir, file.as_str())
                    .map(Reply::from)
                    .map_err(|error| error.to_string());
            }
        };
        let project = load(dir)?;
        let id = AssetId::new(page.as_str());
        let asset = project
            .asset(&id)
            .ok_or_else(|| format!("there is no asset `{id}`; page_write starts a page"))?;
        let path = match (asset.kind, &asset.path) {
            (AssetKind::Html, Some(path)) => path,
            _ => return Err(format!("`{id}` is not a page")),
        };
        path.check()
            .map_err(|problem| format!("the page `{id}` is at {path}, which {problem}"))?;
        std::fs::read_to_string(path.resolve(dir))
            .map(Reply::from)
            .map_err(|error| format!("the page `{id}` at {path} could not be read: {error}"))
    }
}
