//! `stock_import` — bring a chosen result in as an ordinary asset.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_providers::stock::{self, Choice, Fetched, Kept, Medium, StockError, cache_dir};
use scorsese_render::{Ffprobe, Resolution};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Importing from Pixabay, or a Lottie from LottieFiles.
pub(crate) struct Import;

/// One id or several.
#[derive(Deserialize)]
#[serde(untagged)]
enum Ids {
    /// `"id": 39009`.
    One(u64),
    /// `"id": [39009, 35693]`.
    Many(Vec<u64>),
}

impl JsonSchema for Ids {
    fn schema_name() -> Cow<'static, str> {
        "Ids".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": ["integer", "array"],
            "items": { "type": "integer" }
        })
    }
}

/// What `stock_import` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The id a stock_search result named, or a list of them — all of one
    /// kind.
    id: Ids,
    /// video (the default), image or lottie: the kind the search was for.
    /// Ids are only unique within a kind.
    #[schemars(extend("enum" = ["video", "image", "lottie"]))]
    kind: Option<String>,
    /// The size the video will be rendered at, e.g. 1080x1920 for a vertical
    /// cut. Default 1920x1080. The smallest file that fills it without being
    /// enlarged is downloaded, or the largest there is when none does. Not
    /// for lottie, which is vector and sharp at any size.
    resolution: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("id", "the id a stock_search result named")];
}

impl Tool for Import {
    fn name(&self) -> &'static str {
        "stock_import"
    }

    fn description(&self) -> &'static str {
        "Bring stock footage, a photo or a Lottie animation that stock_search \
         found into the project, by its id and kind. FREE. Footage and photos \
         become ordinary video or image assets: downloaded into assets/, \
         probed and hashed like any import, and ready for place_clip, trim, \
         crop, speed and grade. The file is the smallest Pixabay has that \
         fills the render size without being enlarged (pass resolution for \
         anything but 1920x1080), and it lands as assets/pixabay-<id>, so the \
         asset id is pixabay-<id> unless the reply says otherwise. Photos top \
         out at 1280 px wide until Pixabay grants full API access, which is \
         soft full-frame at 1080p; the reply says when a file is smaller than \
         the frame. A lottie is NOT an asset and is never placed by itself: \
         its JSON is written to pages/lottie-<id>.json, and an html page \
         plays it with the shipped lottie-web, driven from the page's clock \
         (`guide pages`, section *A Lottie animation*) — write that page with \
         page_write and place_clip the page. Pass a list of ids to bring \
         several in at once; one that fails costs none of the others."
    }

    fn costs(&self) -> Costs {
        Costs::Request
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let medium = super::medium(arguments.kind.as_deref())?;
        let ids = match arguments.id {
            Ids::One(id) => vec![id],
            Ids::Many(ids) if ids.is_empty() => return Err("`id` is required".to_owned()),
            Ids::Many(ids) => ids,
        };
        let frame = match args::given(arguments.resolution.as_deref()) {
            Some(text) => text
                .parse::<Resolution>()
                .map_err(|problem| format!("resolution: {problem}"))?,
            None => Resolution::HD,
        };
        let mut project = load(dir)?;
        let library = super::library(medium)?;
        if medium == Medium::Lottie {
            return kept(&ids, &stock::keep(dir, &cache_dir(dir), &*library, &ids));
        }
        let probe = Ffprobe::discover().map_err(|error| format!("{error}"))?;
        let choices: Vec<Choice> = ids.iter().map(|&id| Choice { medium, id }).collect();
        let answers = stock::import(
            &mut project,
            dir,
            &cache_dir(dir),
            &*library,
            &choices,
            (frame.width(), frame.height()),
            &probe,
        );
        if answers
            .iter()
            .any(|answer| answer.as_ref().is_ok_and(|one| !one.imported.reused))
        {
            project
                .save(dir)
                .map_err(|error| format!("saving the project: {error}"))?;
        }
        said(&ids, &answers, frame)
    }
}

/// What came in, from where, and what did not.
fn said(
    ids: &[u64],
    answers: &[Result<Fetched, StockError>],
    frame: Resolution,
) -> Result<Reply, String> {
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    for (id, answer) in ids.iter().zip(answers) {
        match answer {
            Ok(one) => lines.push(fetched(one, frame)),
            Err(error) => failures.push(format!("{id} — failed: {error}")),
        }
    }
    if lines.is_empty() {
        return Err(format!("{} — nothing was imported", failures.join("; ")));
    }
    lines.extend(failures);
    lines.push(String::from(
        "From Pixabay, under the Pixabay Content License: free for commercial use, no \
         attribution needed.",
    ));
    Ok(lines.join("\n").into())
}

/// What came in of the animations asked for, and how a page plays each.
fn kept(ids: &[u64], answers: &[Result<Kept, StockError>]) -> Result<Reply, String> {
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    for (id, answer) in ids.iter().zip(answers) {
        match answer {
            Ok(one) => lines.push(animation(one)),
            Err(error) => failures.push(format!("{id} — failed: {error}")),
        }
    }
    if lines.is_empty() {
        return Err(format!("{} — nothing was imported", failures.join("; ")));
    }
    lines.extend(failures);
    if let Some(first) = answers.iter().flatten().next() {
        lines.push(super::player::page(&first.file));
    }
    lines.push(format!(
        "From LottieFiles. {}",
        stock::licence(Medium::Lottie)
    ));
    Ok(lines.join("\n").into())
}

/// One animation's line.
fn animation(one: &Kept) -> String {
    let mut line = format!(
        "{} — \"{}\" by {}, {}x{}, {} frames at {} fps ({:.2}s), {} KB — {}",
        one.path,
        one.candidate.title,
        one.candidate.author,
        one.size.0,
        one.size.1,
        one.frames,
        one.fps,
        one.seconds(),
        one.bytes.div_ceil(1024),
        one.candidate.page_url,
    );
    if one.reused {
        line.push_str("\n   already there, nothing written");
    }
    line.push_str(&format!(
        "\n   from a page in pages/ it is \"{}\"",
        one.file
    ));
    if !one.outside.is_empty() {
        line.push_str(&format!(
            "\n   it names pictures it does not carry ({}), which a page will not find",
            one.outside.join(", ")
        ));
    }
    line
}

/// One import's line.
fn fetched(one: &Fetched, frame: Resolution) -> String {
    let (rendition, id) = (&one.rendition, &one.imported.id);
    let mut line = if one.imported.reused {
        format!("{id} — already in the pool, nothing copied")
    } else {
        format!(
            "{id} — {} {} ({}, {}x{}), by {} — {}",
            one.candidate.medium.word(),
            one.candidate.id,
            rendition.name,
            rendition.width,
            rendition.height,
            one.candidate.author,
            one.candidate.page_url
        )
    };
    if let Some(wanted) = &one.imported.wanted {
        line.push_str(&format!(", renamed: `{wanted}` taken"));
    }
    if !one.fills {
        line.push_str(&format!(
            "\n   smaller than the {}x{} frame: it is the largest Pixabay offers, and will \
             look soft full-frame",
            frame.width(),
            frame.height()
        ));
    }
    line
}
