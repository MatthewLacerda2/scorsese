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
//! How a page is written well — the contract it is drawn under, what it can
//! load offline, why seekable animation beats an integrated loop — is
//! `docs/pages.md`, which the descriptions point at.

use schemars::JsonSchema;
use scorsese_core::{AssetId, AssetKind, write_page};
use serde::Deserialize;
use serde_json::Value;

use super::args::{self, Name, ProjectDir, Required};
use super::inspect::load;
use super::{Costs, Reply, Tool};

/// The `page` argument, described the same way in both tools.
const PAGE: &str = "The page's asset id, e.g. lower-third. project_assets lists the \
                    html assets a project has.";

/// Write a page, making it when the name is new.
pub(super) struct Write;

/// What `page_write` takes.
#[derive(Deserialize, JsonSchema)]
struct WriteArguments {
    project: ProjectDir,
    /// The page's asset id, e.g. lower-third. A name no asset has yet makes
    /// an html asset of that id, its file at pages/<id>.html; the id of a page
    /// already there rewrites that page's file. The id of any other kind of
    /// asset is refused.
    page: Name,
    /// The complete HTML document. Not a patch — whatever is here replaces
    /// the file. docs/pages.md has the contract it is drawn under.
    html: String,
}

impl args::Arguments for WriteArguments {
    const REQUIRED: Required = &[("page", "the page's asset id"), ("html", "the page's HTML")];
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
         window.scorsese gives its width, height, fps and duration; the shipped \
         fonts are there by name and anime.js at \
         https://lib.scorsese/anime.min.js; where it draws nothing, the tracks \
         below show through. Read docs/pages.md before writing one."
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
        let mut project = load(dir)?;
        let html = &arguments.html;
        let written = write_page(&mut project, dir, arguments.page.as_str(), html)
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

/// A page as it is on disk.
pub(super) struct Read;

/// What `page_read` takes.
#[derive(Deserialize, JsonSchema)]
struct ReadArguments {
    project: ProjectDir,
    #[schemars(description = PAGE)]
    page: Name,
}

impl args::Arguments for ReadArguments {
    const REQUIRED: Required = &[("page", "the page's asset id")];
}

impl Tool for Read {
    fn name(&self) -> &'static str {
        "page_read"
    }

    fn description(&self) -> &'static str {
        "Read a web page's HTML exactly as it is on disk. Pair with page_write \
         to change one: read it, change it, write it back, look with still."
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
        let project = load(dir)?;
        let id = AssetId::new(arguments.page.as_str());
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
