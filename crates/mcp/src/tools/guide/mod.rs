//! `guide` — the agent-facing pages of `docs/`, served as a tool (#909).
//!
//! Tool descriptions are paid for on every model call, so the long *how* of a
//! craft — the contract a page is drawn under, the recipe format, what a word
//! in a prompt makes a provider do — lives in a guide rather than in them, and
//! the descriptions point at it. A pointer to `docs/pages.md` works only for a
//! client sitting in a checkout of this repository; the web assistant, a
//! user's own client on web MCP and anyone running an installed build have no
//! such file. So the pointer names this tool instead (`` `guide pages` ``),
//! and the tool hands back the page.
//!
//! The guides themselves, and how a long one is cut into sections, are
//! `scorsese_core::guide`'s: `scorsese guide` in a terminal reads the same
//! pages through the same function, so the two surfaces give the same answer.

#[cfg(test)]
mod tests;

use schemars::JsonSchema;
use scorsese_core::guide;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::args::{self, Name, ProjectDir, Required};
use super::{Costs, Reply, Tool};

/// Read one of the guides.
pub(super) struct Read;

/// What `guide` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    // Required and not read, as `icons` takes it: every tool names the project
    // it is called about, and the guides are the same for all of them.
    #[expect(dead_code, reason = "taken for the uniform surface, not read")]
    project: ProjectDir,
    /// Which guide to read; the tool's description says what each one holds.
    #[schemars(extend("enum" = ["pages", "project-format", "recipes", "references", "prompts", "prices", "stock"]))]
    name: Name,
    /// Only this section: its heading's words (or the only heading containing
    /// them), or its number in the list a long guide answers with. Left out,
    /// the whole guide when it is short, its opening and list of sections when
    /// it is long.
    #[serde(default, deserialize_with = "section")]
    section: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("name", "which guide, like `pages`")];
}

/// A section as text, or as the bare number a client may send for one from
/// the list — both mean the same heading.
fn section<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Ok(match Option::<Value>::deserialize(deserializer)? {
        Some(Value::String(text)) => args::given(Some(&text)).map(str::to_owned),
        Some(Value::Number(number)) => Some(number.to_string()),
        Some(Value::Null) | None => None,
        Some(_) => return Err(serde::de::Error::custom("a heading or its number")),
    })
}

impl Tool for Read {
    fn name(&self) -> &'static str {
        "guide"
    }

    fn description(&self) -> &'static str {
        "Read one of scorsese's guides: the how-to that no tool's description \
         has room for. Other tools point here as `guide <name>`, sometimes with \
         a section. pages: writing a web page the timeline plays — the \
         contract it is drawn under, what loads offline, worked pages; read it \
         before page_write. project-format: the project.json document, and the \
         table of what clip_animate can animate. recipes: synthesis recipes, \
         effects and songs. references: what real records measure, to hold a \
         score against. prompts: what certain words make a provider do, and \
         Google's and ElevenLabs' own prompting advice; read it before \
         writing a prompt. prices: the providers' rates, and why a cost \
         is an estimate. stock: when free stock beats a generation. A short \
         guide comes back whole; a long one (recipes, project-format) comes \
         back as its opening and a numbered list of its sections, to call again \
         with `section`. Costs nothing: the guides are compiled into this build."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        guide::read(arguments.name.as_str(), arguments.section.as_deref()).map(Reply::from)
    }
}
