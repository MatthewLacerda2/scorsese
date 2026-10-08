//! `stock_search` — find free stock media, and see it before choosing.

use schemars::JsonSchema;
use scorsese_providers::stock::{
    self, Candidate, Found, LICENCE, Library, Orientation, Query, cache_dir, footage, previews,
};
use scorsese_render::contact::{self, Look, MAX_FRAMES};
use scorsese_render::{Tools, frames};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir};
use crate::tools::scratch::Scratch;
use crate::tools::{Costs, Part, Reply, Tool};

/// Searching Pixabay.
pub(crate) struct Search;

/// What `stock_search` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// What to find, in plain words as a stock site takes them: "city at
    /// night", "hands typing on a laptop", "cat asleep". English finds the most.
    query: Option<String>,
    /// video (the default) for footage, image for photos and illustrations.
    #[schemars(extend("enum" = ["video", "image"]))]
    kind: Option<String>,
    /// Only results this way round. For a vertical edit, vertical; footage
    /// is filtered by its measured size, since Pixabay cannot filter it.
    #[schemars(extend("enum" = ["horizontal", "vertical"]))]
    orientation: Option<String>,
    /// film or animation for video (default film); photo, illustration or
    /// vector for image (default photo).
    style: Option<String>,
    /// Only footage at least this many seconds long.
    min_seconds: Option<u32>,
    /// Which page of results, from 1. Each page is five results, one sheet.
    page: Option<u32>,
    /// Only results suitable for all ages. Default true.
    safe: Option<bool>,
    /// A video id from an earlier search to look through instead of
    /// searching: five frames across the whole shot, from its smallest file,
    /// before importing it. Nothing is imported.
    look: Option<u64>,
}

impl args::Arguments for Arguments {}

impl Arguments {
    /// The search the arguments name.
    fn query(&self) -> Result<Query, String> {
        let words = args::given(self.query.as_deref())
            .ok_or("`query` is required: the words to search for")?;
        let orientation = match args::given(self.orientation.as_deref()) {
            None => None,
            Some("horizontal") => Some(Orientation::Horizontal),
            Some("vertical") => Some(Orientation::Vertical),
            Some(other) => {
                return Err(format!(
                    "`orientation` is horizontal or vertical, not `{other}`"
                ));
            }
        };
        Ok(Query {
            medium: super::medium(self.kind.as_deref())?,
            words: words.to_owned(),
            style: args::given(self.style.as_deref()).map(ToOwned::to_owned),
            orientation,
            min_seconds: self.min_seconds,
            safe: self.safe.unwrap_or(true),
        })
    }
}

impl Tool for Search {
    fn name(&self) -> &'static str {
        "stock_search"
    }

    fn description(&self) -> &'static str {
        "Search Pixabay's free stock footage and photos, and see the candidates \
         before choosing. FREE — no money, no quote — so for a generic shot (a \
         city at night, hands on a keyboard, a sunrise, an office, a cat asleep) \
         reach for this before generate, and keep generate for shots that have \
         to be unique. Answers five results a page, each with its id, length, \
         largest size, tags and Pixabay page, and ONE contact sheet of their \
         previews numbered in order: look at it, because tags alone pick the \
         wrong shot. For a video, pass look with its id to see five frames \
         across the whole shot before importing. Then stock_import the chosen \
         id. Results come from Pixabay and are cached for 24 hours. The \
         Pixabay Content License allows commercial use with no attribution; \
         identifiable people, logos or brands in a commercial video may need \
         consent, which is the user's to get."
    }

    fn costs(&self) -> Costs {
        Costs::Request
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let cache = cache_dir(arguments.project.dir());
        let library = super::library()?;
        if let Some(id) = arguments.look {
            return looked(&cache, &library, id);
        }
        let query = arguments.query()?;
        let found = stock::search(&cache, &library, &query, arguments.page.unwrap_or(1))
            .map_err(|error| format!("{error}"))?;
        let mut text = listed(&found);
        if found.candidates.is_empty() {
            return Ok(text.into());
        }
        match sheet(&cache, &library, &found.candidates, query.orientation) {
            Ok(png) => Ok(vec![Part::picture(text, &png)].into()),
            Err(why) => {
                text.push_str(&format!("\n(No sheet of previews: {why}.)"));
                Ok(text.into())
            }
        }
    }
}

/// The results in words — everything the sheet shows, for a client that
/// cannot see it.
fn listed(found: &Found) -> String {
    let mut lines: Vec<String> = found
        .candidates
        .iter()
        .enumerate()
        .map(|(index, one)| {
            format!(
                "{}. {}\n   by {} — {}",
                index + 1,
                one.says(),
                one.author,
                one.page_url
            )
        })
        .collect();
    if lines.is_empty() {
        lines.push(String::from(
            "Nothing matched. Try fewer or plainer words, or drop a filter.",
        ));
    }
    lines.push(format!("\n{}", found.summary()));
    if !found.candidates.is_empty() {
        lines.push(String::from(
            "Import with stock_import (kind and id); look through a video first with \
             look: <id>.",
        ));
        lines.push(LICENCE.to_owned());
    }
    lines.join("\n")
}

/// One contact sheet of the results' previews, as PNG bytes.
fn sheet(
    cache: &std::path::Path,
    library: &dyn Library,
    candidates: &[Candidate],
    orientation: Option<Orientation>,
) -> Result<Vec<u8>, String> {
    let tools = Tools::discover().map_err(|error| format!("{error}"))?;
    let pictures: Vec<(std::path::PathBuf, String)> = previews(cache, library, candidates)
        .into_iter()
        .zip(candidates.iter().enumerate())
        .filter_map(|(path, (index, one))| Some((path?, one.label(index))))
        .take(MAX_FRAMES)
        .collect();
    if pictures.is_empty() {
        return Err(String::from("no preview would download"));
    }
    let vertical = orientation == Some(Orientation::Vertical);
    let image =
        contact::pictures(&tools, &pictures, vertical).map_err(|error| format!("{error}"))?;
    png(&tools, &image)
}

/// A frame as PNG bytes, through a scratch file — PNG encoding is ffmpeg's.
fn png(tools: &Tools, image: &scorsese_render::Frame) -> Result<Vec<u8>, String> {
    let file = Scratch::at(None);
    frames::write_png(tools, &file.path, image).map_err(|error| format!("{error}"))?;
    std::fs::read(&file.path).map_err(|error| format!("reading the sheet back: {error}"))
}

/// Five frames across video `id`, from its smallest file.
fn looked(cache: &std::path::Path, library: &dyn Library, id: u64) -> Result<Reply, String> {
    let (candidate, file) = footage(cache, library, id).map_err(|error| format!("{error}"))?;
    let tools = Tools::discover().map_err(|error| format!("{error}"))?;
    let seconds = f64::from(candidate.seconds.unwrap_or_default());
    let range = Look {
        to_seconds: (seconds > 0.0).then_some(seconds),
        ..Look::default()
    };
    let sheet = contact::sheet(&tools, &file, &range).map_err(|error| format!("{error}"))?;
    let moments: Vec<String> = sheet
        .at_seconds
        .iter()
        .map(|at| contact::label(*at))
        .collect();
    let text = format!(
        "{}\n   by {} — {}\nFrames at {} of its {:.0}s, from Pixabay's smallest file of it. \
         Import with stock_import (kind video, id {id}).",
        candidate.says(),
        candidate.author,
        candidate.page_url,
        moments.join(", "),
        sheet.duration_seconds,
    );
    Ok(vec![Part::picture(text, &png(&tools, &sheet.image)?)].into())
}
