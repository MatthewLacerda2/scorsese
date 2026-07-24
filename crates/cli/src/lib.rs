//! # scorsese-cli — the headless binary
//!
//! Responsibility: the `scorsese` command-line surface — `new`, `import`,
//! `render`, `generate`, `assets`, `diff`. This is how an agent (or a CI
//! job) assembles and renders a video with no human and no screen: every
//! editing capability the GUI will ever have must be reachable from here
//! first.
//!
//! Boundary: this crate must NEVER touch a display — no window, no GPU
//! surface, no GUI toolkit. It is glue: argument parsing and orchestration
//! over `scorsese-core`, `scorsese-render`, and `scorsese-providers`. Logic
//! that another caller (the MCP server, the GUI) would want lives in those
//! crates, not here.

/// Placeholder so `cargo test` exercises this crate from day one.
/// Replaced by real CLI tests as subcommands land.
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
