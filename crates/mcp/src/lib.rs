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

/// Placeholder so `cargo test` exercises this crate from day one.
/// Replaced by real tool tests in the MCP server issue.
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
