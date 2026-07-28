//! The `scorsese-app` binary: the eframe bootstrap around the window.
//!
//! Everything the window *is* lives in the library beside this. Here there is
//! only an event loop, so that the drawing can be exercised without one — see
//! `scorsese_app::Scorsese::draw`.

use eframe::NativeOptions;

use scorsese_app::Scorsese;

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
