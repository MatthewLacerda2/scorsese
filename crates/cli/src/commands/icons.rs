//! `scorsese icons` — which symbol is the one you mean.
//!
//! The command-line half of the same lookup the MCP server exposes, over the
//! same function: a person picking an icon by hand wants what an assistant
//! wants, and a second implementation is how the two come to disagree about
//! what `film` matches.
//!
//! It touches nothing — no project, no network, no file. The catalogue is
//! compiled into the binary, so this answers in a terminal with no project
//! anywhere near it, which is exactly when somebody is deciding what to build.

use anyhow::Result;
use scorsese_render::icon;

/// Prints the icons a word describes.
///
/// One line, because the answer is a list of names and a sentence saying how
/// many there were — and the count is the half that stops a capped answer
/// reading as the whole set.
pub(crate) fn run(query: &str) -> Result<()> {
    println!("{}", icon::search(query));
    Ok(())
}
