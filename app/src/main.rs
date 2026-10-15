//! The `scorsese-app` binary: the eframe bootstrap around the window.
//!
//! Everything the window *is* lives in the library beside this. Here there is
//! only an event loop, so that the drawing can be exercised without one — see
//! `scorsese_app::Scorsese::draw`.

use std::sync::Arc;

use eframe::NativeOptions;

use scorsese_app::{Launch, Scorsese};

/// How big the window opens. Wide enough that a preview and a timeline both
/// have room, small enough to fit a laptop.
const WINDOW: [f32; 2] = [1280.0, 800.0];

/// The logo, at the one size this binary needs it.
///
/// Committed decoded-from rather than generated, because it is artwork: see
/// `assets/README.md` for where it came from and how to redo it.
const ICON: &[u8] = include_bytes!("../assets/icon.png");

fn main() -> eframe::Result {
    // A path argument, so development does not mean clicking through a file
    // dialog on every run; the usage `--help` prints says so (#1204). Anything
    // else starting with a dash is refused rather than opened as a project.
    let opened = match scorsese_app::launch(std::env::args().skip(1)) {
        Launch::Window(opened) => opened,
        Launch::Say(text) => {
            println!("{text}");
            return Ok(());
        }
        Launch::Refuse(text) => {
            eprintln!("{text}");
            std::process::exit(2);
        }
    };
    // An `html` clip downloads the page renderer the first time one is drawn
    // (#776), with how far it got said under the preview.
    scorsese_app::fetch_on_first_use();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(WINDOW)
        .with_min_inner_size([800.0, 500.0])
        .with_title("scorsese")
        // Wayland does not take an icon from the window the way X11 does — it
        // looks up a desktop entry by this id instead. Setting it costs a line
        // and is what lets a future installed `scorsese.desktop` be found;
        // without it there is nothing for such a file to match on.
        .with_app_id("scorsese");
    if let Some(icon) = icon() {
        viewport = viewport.with_icon(icon);
    }

    eframe::run_native(
        "scorsese",
        NativeOptions {
            viewport,
            ..NativeOptions::default()
        },
        Box::new(move |_cc| Ok(Box::new(Scorsese::opening(opened)))),
    )
}

/// The window's icon: what the taskbar, the switcher and the dock show.
///
/// Decoded once, at startup, from bytes compiled into the binary — so there is
/// no file to find at runtime and no way for a moved install to lose its
/// picture.
///
/// **A failure here costs the icon and nothing else.** The only way this does
/// not decode is a corrupt checkout, which is not something the person opening
/// the window can act on, and a window that opens plain is better than one that
/// refuses to open over a picture. The reason is printed rather than swallowed,
/// because a silently missing icon looks exactly like one nobody wired up.
fn icon() -> Option<Arc<egui::IconData>> {
    match eframe::icon_data::from_png_bytes(ICON) {
        Ok(icon) => Some(Arc::new(icon)),
        Err(error) => {
            eprintln!("the window icon did not decode, so there is none: {error}");
            None
        }
    }
}
