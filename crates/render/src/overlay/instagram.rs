//! Instagram's Reels and Stories, drawn generically: the controls where the
//! app puts them, with no logo and no name of the app's own.

use super::chrome::{self, NAV};
use super::pen::{GLASS, INK, Pen, SOFT, WHITE};

/// A Reel, posted or an ad: header, action rail, profile row, caption, sound,
/// an ad's call to action, and the app's navigation.
pub(super) fn reels(pen: &mut Pen<'_>, ad: bool) {
    chrome::status(pen);
    pen.text("Reels", (48.0, 190.0), 56.0, (true, WHITE));
    pen.icon("camera", (1000.0, 168.0), 60.0, WHITE);

    // The call to action lifts everything above it by its own height.
    let lift = if ad { NAV + 120.0 } else { NAV };
    let rail_top = pen.up(lift + 840.0);
    let end = chrome::rail(
        pen,
        990.0,
        rail_top,
        &[
            ("heart", "12.4K"),
            ("message-circle", "318"),
            ("send", ""),
            ("ellipsis", ""),
        ],
    );
    pen.outline((962.0, end - 50.0, 56.0, 56.0), 12.0, (WHITE, 4.0));

    let row = pen.up(lift + 250.0);
    chrome::avatar(pen, (84.0, row), 36.0);
    let name_end = pen.text(
        "yourbrand",
        (136.0, row + (if ad { 0.0 } else { 14.0 })),
        38.0,
        (true, WHITE),
    );
    if ad {
        pen.text("Sponsored", (136.0, row + 38.0), 28.0, (false, SOFT));
    }
    chrome::button(pen, "Follow", (name_end + 24.0, row), false);
    pen.text(
        "Your caption goes here, as the feed cuts it… more",
        (48.0, row + 104.0),
        34.0,
        (false, WHITE),
    );
    pen.icon("music", (66.0, row + 160.0), 34.0, WHITE);
    pen.text(
        "yourbrand · Original audio",
        (96.0, row + 172.0),
        30.0,
        (false, WHITE),
    );

    if ad {
        chrome::call_to_action(pen, NAV + 104.0, GLASS);
    }
    chrome::navigation(
        pen,
        &[
            ("house", ""),
            ("search", ""),
            ("square-plus", ""),
            ("clapperboard", ""),
            ("circle-user", ""),
        ],
    );
}

/// An ad between Stories: progress bars, the advertiser's row, a close, and
/// the call to action swiped up from the foot.
pub(super) fn story_ad(pen: &mut Pen<'_>) {
    chrome::status(pen);
    pen.boxed((24.0, 96.0, 1032.0, 6.0), 3.0, SOFT);
    pen.boxed((24.0, 96.0, 344.0, 6.0), 3.0, WHITE);
    chrome::avatar(pen, (78.0, 166.0), 32.0);
    pen.text("yourbrand", (128.0, 160.0), 34.0, (true, WHITE));
    pen.text("Sponsored", (128.0, 198.0), 28.0, (false, SOFT));
    pen.icon("ellipsis", (920.0, 166.0), 52.0, WHITE);
    pen.icon("x", (1002.0, 166.0), 56.0, WHITE);

    let foot = pen.up(250.0);
    pen.icon("chevron-up", (540.0, foot), 48.0, WHITE);
    let width = pen.width_of("Learn more", 36.0, true) + 96.0;
    pen.boxed((540.0 - width / 2.0, foot + 40.0, width, 84.0), 42.0, WHITE);
    pen.centred("Learn more", (540.0, foot + 95.0), 36.0, (true, INK));
}
