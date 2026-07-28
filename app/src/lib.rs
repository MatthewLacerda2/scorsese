//! # scorsese-app — the window
//!
//! The one part of scorsese a person looks at. It opens a `*.scor/` directory,
//! draws what is in it, and lets you make the small direct changes that are
//! faster to do than to describe: scrub, select, nudge, trim, change a value.
//!
//! Everything with structure to it — building a cut, scoring it, generating
//! shots — is a sentence to an assistant over MCP. That is not a limitation
//! being apologised for: a window rich enough to do the editing would be a
//! second and weaker way to do everything, and this one stays thin so it can
//! keep being reshaped from use.
//!
//! ## Boundary
//!
//! This is a **client** of the same library logic the CLI and the MCP server
//! use, not a layer underneath them. It reads and writes the project through
//! `scorsese-core`, which means the document on disk is the only model and the
//! window can never disagree with what the other two see.
//!
//! It is also its own cargo workspace. A graphics stack has no business being
//! compiled by `cargo test --workspace`, which every headless change runs.

//! ## Testable without a window
//!
//! The window's drawing is [`Scorsese::draw`], which takes a `Ui` and nothing
//! else — no `eframe::Frame`, no event loop. That is what lets the panels be
//! rendered offscreen and *looked at* in a test, on a machine with no display
//! at all. The binary is the eframe bootstrap around it and nothing more.

mod editing;
mod files;
mod inspector;
mod preview;
mod project;
mod timeline;
mod ui;

pub use ui::Scorsese;
