//! YouTube Shorts, drawn generically: the controls where the app puts them,
//! with no logo and no name of the app's own.

use super::chrome::{self, NAV};
use super::pen::{Pen, SOFT, WHITE};

/// A Short: search and menu, the action rail, the channel row with its
/// subscribe button, the caption and sound, progress, and navigation.
pub(super) fn draw(pen: &mut Pen<'_>) {
    chrome::status(pen);
    pen.icon("search", (880.0, 160.0), 60.0, WHITE);
    pen.icon("ellipsis-vertical", (1000.0, 160.0), 60.0, WHITE);

    let rail_top = pen.up(NAV + 1010.0);
    let end = chrome::rail(
        pen,
        990.0,
        rail_top,
        &[
            ("thumbs-up", "12K"),
            ("thumbs-down", "Dislike"),
            ("message-square", "318"),
            ("forward", "Share"),
            ("repeat-2", "Remix"),
        ],
    );
    pen.boxed((958.0, end - 40.0, 64.0, 64.0), 12.0, SOFT);

    let row = pen.up(NAV + 260.0);
    chrome::avatar(pen, (84.0, row), 34.0);
    let name_end = pen.text("@yourchannel", (134.0, row + 13.0), 36.0, (true, WHITE));
    chrome::button(pen, "Subscribe", (name_end + 24.0, row), true);
    pen.text(
        "Your caption goes here #shorts",
        (48.0, row + 96.0),
        34.0,
        (false, WHITE),
    );
    pen.icon("music", (66.0, row + 150.0), 32.0, WHITE);
    pen.text("Original sound", (96.0, row + 162.0), 30.0, (false, WHITE));

    pen.boxed((0.0, pen.up(NAV + 6.0), 1080.0, 6.0), 0.0, SOFT);
    chrome::navigation(
        pen,
        &[
            ("house", "Home"),
            ("zap", "Shorts"),
            ("circle-plus", ""),
            ("square-play", "Subscriptions"),
            ("circle-user", "You"),
        ],
    );
}
