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

mod project;
mod ui;

use eframe::NativeOptions;

use ui::Scorsese;

/// How big the window opens. Wide enough that a preview and a timeline both
/// have room, small enough to fit a laptop.
const WINDOW: [f32; 2] = [1280.0, 800.0];

fn main() -> eframe::Result {
    // A path argument, so development does not mean clicking through a file
    // dialog on every run. Not a documented flag: the shipped way in is the
    // dialog, and this is a convenience for whoever is building the thing.
    let opened = std::env::args().nth(1).map(std::path::PathBuf::from);

    eframe::run_native(
        "scorsese",
        NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(WINDOW)
                .with_min_inner_size([800.0, 500.0])
                .with_title("scorsese"),
            ..NativeOptions::default()
        },
        Box::new(move |_cc| Ok(Box::new(Scorsese::opening(opened)))),
    )
}
