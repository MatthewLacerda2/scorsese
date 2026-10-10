//! `vectorize` — a picture traced into an SVG a page draws on (#999).
//!
//! The missing step between a generated illustration and a board that draws
//! itself: Gemini draws pixels, `kit.draw` draws paths. The tracing is
//! `scorsese_render::trace`'s; this is the wiring, the words, and the page that
//! draws the result, handed back so the next call can be `page_write`.

use schemars::JsonSchema;
use scorsese_render::Tools;
use scorsese_render::trace::{Detail, Tracing, vectorize};
use serde::Deserialize;
use serde_json::Value;

use super::args::{self, Name, ProjectDir, Required};
use super::inspect::load;
use super::{Costs, Reply, Tool};

/// Trace a picture into `pages/<name>.svg`.
pub(crate) struct Vectorize;

/// What `vectorize` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The picture's asset id: an image, or a generated_image that has been
    /// generated.
    asset: Name,
    /// What to call the drawing: the file is pages/<name>.svg and its group of
    /// marks has id <name>. Letters, digits, - and _. Default: the asset id.
    /// Tracing again under the same name replaces the file.
    name: Option<String>,
    /// At most this many colours, the background included: 2 to 32, default
    /// 8. A flat illustration needs as many as it has; more only traces
    /// shading as bands.
    colours: Option<usize>,
    /// How much small detail survives: low (bold shapes, small marks dropped,
    /// curves smoothed), medium (the default) or high (small marks kept,
    /// curves followed closely, more paths).
    #[schemars(extend("enum" = ["low", "medium", "high"]))]
    detail: Option<String>,
    /// Keep the picture's background colour as a shape of its own. Default
    /// false: the background is dropped and the page shows around the drawing.
    keep_background: Option<bool>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("asset", "the picture's asset id")];
}

impl Tool for Vectorize {
    fn name(&self) -> &'static str {
        "vectorize"
    }

    fn description(&self) -> &'static str {
        "Trace a picture into an SVG a page draws on, stroke by stroke. It takes \
         an image asset, or a generated_image once generated, and makes what \
         the kit's kit.draw draws for the whiteboard 'board that draws itself'. FREE: local, offline, no key \
         and no quote. The usual loop is asset_set a generated_image sketch in a \
         flat illustration style, generate, vectorize, then page_write a page that \
         draws it — the reply hands that page back. The SVG is written to \
         pages/<name>.svg, beside the pages, and is not an asset: the page is \
         what goes on the timeline. Its marks come in the order a hand would \
         draw them: the dark outlines first, each one a single pen stroke, top \
         to bottom, then each colour sketched and filled, largest first. The \
         background is dropped unless kept, so the drawing sits on the page's \
         board. Made for flat art — a character, a logo, a scanned drawing; a \
         photograph traces into a mosaic of blobs. The prompt that traces \
         cleanly was tested 2026-10-10: end it with 'flat vector illustration, \
         thick dark outlines, solid colours, no gradients or shading, plain \
         white background' — without naming the illustration, Nano Banana \
         draws a photograph. `guide prompts`, section 'A picture to trace is \
         asked for flat', has the comparison; `guide pages` has the worked page."
    }

    fn costs(&self) -> Costs {
        Costs::Decode
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let asset = arguments.asset.as_str();
        let name = args::given(arguments.name.as_deref()).unwrap_or(asset);
        let tracing = tracing(&arguments)?;
        let project = load(dir)?;
        let tools = Tools::discover().map_err(|error| format!("{error}"))?;
        let written = vectorize(&tools, &project, dir, asset, name, tracing)
            .map_err(|error| format!("nothing written — {error}"))?;
        Ok(format!("{}\n{}", written.summary(), page(name)).into())
    }
}

/// The choices, checked.
fn tracing(arguments: &Arguments) -> Result<Tracing, String> {
    let mut tracing = Tracing::default();
    if let Some(colours) = arguments.colours {
        if !(2..=32).contains(&colours) {
            return Err(format!("`colours` is 2 to 32, not {colours}"));
        }
        tracing.colours = colours;
    }
    if let Some(word) = args::given(arguments.detail.as_deref()) {
        tracing.detail = Detail::named(word)
            .ok_or_else(|| format!("`detail` is low, medium or high, not `{word}`"))?;
    }
    tracing.keep_background = arguments.keep_background.unwrap_or(false);
    Ok(tracing)
}

/// The page that draws `name`.svg on over four seconds, to write with
/// page_write as it is or inside a larger page.
fn page(name: &str) -> String {
    format!(
        "Draw it on from an html page beside it — this one draws it over four seconds, \
         600 px tall in the middle of the frame; size and place #art as you like, or \
         time it to the narration (`guide pages`, *A traced picture, drawn on*):\n\
         <!doctype html><html><head><script src=\"https://lib.scorsese/kit.js\"></script>\n\
         <style>html, body {{ margin: 0; height: 100%; }} body {{ display: grid; \
         place-content: center; }} #art svg {{ height: 600px; width: auto; }}</style></head>\n\
         <body><div id=\"art\"></div><script>\n\
         fetch(\"{name}.svg\").then((r) => r.text()).then((svg) => {{\n\
         \x20 document.getElementById(\"art\").innerHTML = svg;\n\
         \x20 const drawing = document.getElementById(\"{name}\");\n\
         \x20 kit.frame((t) => kit.draw(drawing, t, 0, {{ within: 4 }}));\n\
         }});\n\
         </script></body></html>"
    )
}
