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
//! **Each guide is the file itself**, compiled in with `include_str!` — not a
//! copy, not a summary — so what the tool says cannot drift from what the
//! repository says, and every gate the file is held to (the format page's
//! examples parsed as projects, its animatable-property table held to the
//! code, the recipe examples held to the parser) holds what this serves.
//!
//! Only the pages written for whoever is *making a video* are here. The
//! developer docs — the web app's internals, the golden-render rulebook,
//! mutation testing — are about building scorsese, and an agent editing a film
//! has no use for them.

mod outline;

#[cfg(test)]
mod tests;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::args::{self, Name, ProjectDir, Required};
use super::{Costs, Reply, Tool};

/// One guide: the name a call gives, and the page.
struct Guide {
    /// What `name` is set to, and what other tools write after `guide`.
    name: &'static str,
    /// The page, as the repository has it.
    text: &'static str,
}

/// Every guide this build serves, in the order the description lists them.
const GUIDES: &[Guide] = &[
    Guide {
        name: "pages",
        text: include_str!("../../../../../docs/pages.md"),
    },
    Guide {
        name: "project-format",
        text: include_str!("../../../../../docs/project-format.md"),
    },
    Guide {
        name: "recipes",
        text: include_str!("../../../../../docs/recipes.md"),
    },
    Guide {
        name: "references",
        text: include_str!("../../../../../docs/references.md"),
    },
    Guide {
        name: "prompts",
        text: include_str!("../../../../../docs/prompts.md"),
    },
    Guide {
        name: "prices",
        text: include_str!("../../../../../docs/prices.md"),
    },
    Guide {
        name: "stock",
        text: include_str!("../../../../../docs/stock.md"),
    },
];

/// Read one of the guides.
pub(super) struct Read;

/// What `guide` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    // Required and not read, as `icons` takes it: every tool names the project
    // it is called about, and the guides are the same for all of them.
    #[expect(dead_code, reason = "taken for the uniform surface, not read")]
    project: ProjectDir,
    /// Which guide.
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
         score against. prompts: what certain words make a provider do; read it \
         before writing a prompt. prices: the providers' rates, and why a cost \
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
        read(arguments.name.as_str(), arguments.section.as_deref()).map(Reply::from)
    }
}

/// The guide called `name`, or the part of it `section` names.
fn read(name: &str, section: Option<&str>) -> Result<String, String> {
    let guide = GUIDES
        .iter()
        .find(|guide| guide.name == name)
        .ok_or_else(|| {
            let names: Vec<&str> = GUIDES.iter().map(|guide| guide.name).collect();
            format!("there is no guide `{name}`; there are {}", names.join(", "))
        })?;
    let text = guide.text;
    let headings = outline::headings(text);
    let Some(asked) = section else {
        return Ok(outline::part(
            text,
            &headings,
            0,
            text.len(),
            &format!("guide {name}"),
        ));
    };
    let heading = outline::find(&headings, asked).map_err(|why| format!("guide {name}: {why}"))?;
    Ok(outline::part(
        text,
        &headings,
        heading.start,
        heading.end,
        &format!("\"{}\"", heading.title),
    ))
}
