//! `scorsese guide` — the agent-facing pages of `docs/`, for an installed build.
//!
//! A message that sent its reader to a page under `docs/` helped only somebody
//! sitting in a checkout of this repository (#916). The pages are compiled
//! into the binary instead, and this prints them through the same function
//! the MCP server's `guide` tool answers with, so a terminal and an assistant
//! are never told two different things.

use anyhow::{Result, anyhow};
use scorsese_core::guide;

/// Prints the guide called `name`, or the part of it `section` names.
pub(crate) fn run(name: &str, section: Option<&str>) -> Result<()> {
    println!(
        "{}",
        guide::read(name, section).map_err(|why| anyhow!(why))?
    );
    Ok(())
}
