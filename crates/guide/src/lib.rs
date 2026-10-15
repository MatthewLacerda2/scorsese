//! # scorsese-guide — the agent-facing pages of `docs/`, compiled in
//!
//! Responsibility: handing back a guide, or the part of one a caller names
//! (#909, #916). Tool descriptions and command help are paid for or skimmed on
//! every use, so the long *how* of a craft — the contract a page is drawn
//! under, the recipe format, what a word in a prompt makes a provider do —
//! lives in a guide, and they point at it. A pointer to `docs/pages.md` works
//! only for someone sitting in a checkout of this repository; the web
//! assistant, a user's own client on web MCP and anyone running an installed
//! build have no such file. So a pointer names the guide instead — `guide
//! pages` to an MCP client, `scorsese guide pages` in a terminal — and both
//! hand back the page from here, so the two surfaces cannot answer
//! differently.
//!
//! The guides follow the same rule among themselves (#936): one points at
//! another as `` `guide <name>` ``, with `, section "<words>"` where a part is
//! meant — the words a call and a command share, which a terminal reader runs
//! as `scorsese guide <name>` — and keeps a Markdown link beside it only for
//! someone reading the repository. The tests in `pointers` hold that.
//!
//! **Each guide is the file itself**, compiled in with `include_str!` — not a
//! copy, not a summary — so what is served cannot drift from what the
//! repository says, and every gate the file is held to (the format page's
//! examples parsed as projects, its animatable-property table held to the
//! code, the recipe examples held to the parser) holds what this serves.
//!
//! Only the pages written for whoever is *making a video* are here. The
//! developer docs — the web app's internals, the golden-render rulebook,
//! mutation testing — are about building scorsese, and an agent editing a film
//! has no use for them.
//!
//! ## Boundary
//!
//! A leaf, and it stays one (#1129): this crate depends on **nothing** — not
//! `scorsese-core`, not any other crate of ours, not a crates.io dependency —
//! and performs **no I/O**: no file is read at run time and nothing is
//! spawned. It is text and arithmetic over text. That is the point of it being
//! its own crate: these pages change several times a batch, and while they
//! were compiled into `core`, the root of the graph, editing any of them
//! rebuilt every crate and relinked every test binary. Here an edit rebuilds
//! this crate and the two that serve it, `scorsese-cli` and `scorsese-mcp`
//! (and the server, through `mcp`). A test that holds another crate's text to
//! a guide — a style's prompt naming a section — lives in a crate that
//! depends on both, never here.

mod outline;

#[cfg(test)]
mod pointers;
#[cfg(test)]
mod tests;

/// One guide: the name a call gives, and the page.
struct Guide {
    /// What a caller names it by, and what a pointer writes after `guide`.
    name: &'static str,
    /// The page, as the repository has it.
    text: &'static str,
    /// The longest part of it handed back whole, in bytes.
    limit: usize,
}

/// Every guide this build serves, in the order they are listed.
const GUIDES: &[Guide] = &[
    Guide {
        name: "pages",
        text: include_str!("../../../docs/pages.md"),
        limit: outline::PAGES_LIMIT,
    },
    Guide {
        name: "project-format",
        text: include_str!("../../../docs/project-format.md"),
        limit: outline::LIMIT,
    },
    Guide {
        name: "recipes",
        text: include_str!("../../../docs/recipes.md"),
        limit: outline::LIMIT,
    },
    Guide {
        name: "references",
        text: include_str!("../../../docs/references.md"),
        limit: outline::LIMIT,
    },
    Guide {
        name: "prompts",
        text: include_str!("../../../docs/prompts.md"),
        limit: outline::LIMIT,
    },
    Guide {
        name: "prices",
        text: include_str!("../../../docs/prices.md"),
        limit: outline::LIMIT,
    },
    Guide {
        name: "stock",
        text: include_str!("../../../docs/stock.md"),
        limit: outline::LIMIT,
    },
    Guide {
        name: "styles",
        text: include_str!("../../../docs/styles.md"),
        limit: outline::LIMIT,
    },
];

/// The name of every guide, in the order they are listed — what a schema
/// offers and what a refusal names.
pub fn names() -> impl Iterator<Item = &'static str> {
    GUIDES.iter().map(|guide| guide.name)
}

/// The guide called `name`, or the part of it `section` names.
///
/// A part short enough comes back exactly as the file has it; a longer one
/// comes back as its opening and a numbered map of the headings inside it.
/// `section` is a heading's words (or the only heading containing them), or
/// its number in that map. The error is a sentence for whoever asked, listing
/// what there is instead.
pub fn read(name: &str, section: Option<&str>) -> Result<String, String> {
    let guide = GUIDES
        .iter()
        .find(|guide| guide.name == name)
        .ok_or_else(|| {
            let names: Vec<&str> = names().collect();
            format!("there is no guide `{name}`; there are {}", names.join(", "))
        })?;
    let text = guide.text;
    let headings = outline::headings(text);
    let Some(asked) = section else {
        return Ok(outline::part(
            text,
            &headings,
            (0, text.len()),
            guide.limit,
            &format!("guide {name}"),
        ));
    };
    let heading = outline::find(&headings, asked).map_err(|why| format!("guide {name}: {why}"))?;
    Ok(outline::part(
        text,
        &headings,
        (heading.start, heading.end),
        guide.limit,
        &format!("\"{}\"", heading.title),
    ))
}
