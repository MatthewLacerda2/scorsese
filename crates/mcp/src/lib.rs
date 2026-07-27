//! # scorsese-mcp — the MCP server (skeleton)
//!
//! Responsibility: exposing scorsese to Claude agents as MCP tools —
//! plan/import/generate/render/diff — as a thin wrapper over the same
//! library logic the CLI uses. If a tool here needs code the CLI doesn't
//! share, that code is in the wrong place: push it down into `core`,
//! `render`, or `providers`.
//!
//! Boundary: protocol handling only. No editing logic of its own, no
//! display, no direct ffmpeg or provider calls — everything goes through
//! the lower crates.
//!
//! **Every tool exposed here carries a description, and that is enforced by a
//! test from the first tool onwards.** A tool's description is not a courtesy:
//! it is the entire interface an agent has to it, and an undescribed tool is a
//! capability that exists and cannot be found — nothing fails, the agent
//! simply never calls it. The same gate already covers the CLI's `--help` (see
//! `crates/cli/tests/help.rs`) and the property table in
//! `docs/project-format.md`; walking a tool registry is the same test again.
//! Building it in with the first tool costs one test. Retrofitting it once
//! there are twelve tools, three of them undescribed, costs an argument about
//! which three.

/// Placeholder so `cargo test` exercises this crate from day one.
/// Replaced by real tool tests in the MCP server issue.
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
