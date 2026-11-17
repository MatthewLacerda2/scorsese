//! TikTok, drawn generically: the controls where the app puts them, with no
//! logo, no brand colour and no name of the app's own.

use super::chrome::{self, NAV};
use super::pen::{GLASS, INK, Pen, SOFT, WHITE};

/// A TikTok, posted or an ad: the feed tabs, the rail with the creator's
/// picture and the spinning record, the caption and sound, an ad's label and
/// call to action, and navigation.
pub(super) fn draw(pen: &mut Pen<'_>, ad: bool) {
    chrome::status(pen);
    pen.icon("tv", (66.0, 160.0), 56.0, WHITE);
    pen.centred("Following", (420.0, 175.0), 38.0, (true, SOFT));
    pen.centred("For You", (650.0, 175.0), 38.0, (true, WHITE));
    pen.boxed((620.0, 196.0, 60.0, 6.0), 3.0, WHITE);
    pen.icon("search", (1010.0, 160.0), 56.0, WHITE);

    let lift = if ad { NAV + 110.0 } else { NAV };
    let creator = pen.up(lift + 1000.0);
    chrome::avatar(pen, (990.0, creator), 48.0);
    pen.disc((990.0, creator + 50.0), 20.0, WHITE);
    pen.icon("plus", (990.0, creator + 50.0), 28.0, INK);
    let end = chrome::rail(
        pen,
        990.0,
        creator + 160.0,
        &[
            ("heart", "12.4K"),
            ("message-circle", "318"),
            ("bookmark", "1,024"),
            ("forward", "Share"),
        ],
    );
    pen.ring((990.0, end - 20.0), 40.0, (SOFT, 14.0));
    pen.icon("music", (990.0, end - 20.0), 30.0, WHITE);

    let row = pen.up(lift + 220.0);
    pen.text("@yourbrand", (40.0, row), 38.0, (true, WHITE));
    pen.text(
        "Your caption goes here #fyp",
        (40.0, row + 56.0),
        34.0,
        (false, WHITE),
    );
    let sound = if ad {
        "Sponsored"
    } else {
        "original sound - yourbrand"
    };
    pen.icon("music", (56.0, row + 104.0), 30.0, WHITE);
    pen.text(sound, (84.0, row + 114.0), 30.0, (false, WHITE));

    if ad {
        chrome::call_to_action(pen, NAV + 100.0, GLASS);
    }
    let foot = pen.up(NAV);
    chrome::navigation(
        pen,
        &[
            ("house", "Home"),
            ("users", "Friends"),
            ("", ""),
            ("message-square", "Inbox"),
            ("user", "Profile"),
        ],
    );
    pen.boxed((488.0, foot + 42.0, 104.0, 64.0), 16.0, WHITE);
    pen.icon("plus", (540.0, foot + 74.0), 40.0, INK);
}
