# app/ — Tauri GUI (placeholder)

The human-facing GUI lives here: a Tauri shell with a timeline view for
scrubbing, tweaking, and reviewing what agents assemble headlessly.

Nothing is built yet — the GUI framework details are still under discussion
(see the "Tauri GUI shell + timeline view" seed issue, which carries the
`planning` label). This directory is deliberately outside the Cargo workspace
so `cargo build`/`cargo test` stay GUI-free; the Tauri scaffold (`src-tauri/`,
frontend) arrives with that issue.

Two invariants already decided:

- The GUI renders previews through `scorsese-compositor` at reduced
  resolution — no second rendering path.
- `core` and `cli` never touch a display; the GUI is a *client* of the same
  library logic the CLI and MCP server use.
