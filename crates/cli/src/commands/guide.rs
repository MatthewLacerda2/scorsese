//! `scorsese guide` — the agent-facing pages of `docs/`, for an installed build.
//!
//! A message that sent its reader to a page under `docs/` helped only somebody
//! sitting in a checkout of this repository (#916). The pages are compiled
//! into the binary instead, and this prints them through the same function
//! the MCP server's `guide` tool answers with, so a terminal and an assistant
//! are never told two different things — `style::read_guide`, which answers a
//! style's or a placement's id under `guide styles` with its recipe (#1531).

use anyhow::{Result, anyhow};
use scorsese_core::style;
use scorsese_guide as guide;

/// Prints the guide called `name`, or the part of it `section` names.
pub(crate) fn run(name: &str, section: Option<&str>) -> Result<()> {
    println!(
        "{}",
        style::read_guide(name, section, guide::read).map_err(|why| anyhow!(why))?
    );
    Ok(())
}
