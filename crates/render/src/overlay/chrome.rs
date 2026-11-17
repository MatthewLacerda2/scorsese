//! What every feed's interface has: the shade its controls sit on, the phone's
//! status bar, a column of actions, a row of navigation, and the tint of the
//! vendor's safe zones.

use scorsese_core::Rgba;
use scorsese_core::style::Margins;

use super::pen::{AVATAR, INK, Pen, SOFT, WHITE, rgba};

/// The covered bands' tint: plainly a warning, light enough to see through.
const TINT: Rgba = rgba(235, 40, 40, 80);
/// The clear area's edge.
const EDGE: Rgba = rgba(235, 40, 40, 230);
/// The darkest the shade under the controls gets.
const SHADE: Rgba = rgba(0, 0, 0, 120);
/// Where the shade fades from.
const NONE: Rgba = rgba(0, 0, 0, 0);

/// The height of a navigation bar.
pub(super) const NAV: f32 = 150.0;

/// The shade at the top and the foot of the screen.
pub(super) fn shade(pen: &mut Pen<'_>) {
    pen.fade(0.0, 300.0, (SHADE, NONE));
    let top = pen.up(900.0);
    pen.fade(top, 900.0, (NONE, SHADE));
}

/// The vendor's covered bands tinted, and the clear area outlined.
pub(super) fn tint(pen: &mut Pen<'_>, margins: &Margins) {
    let (width, height) = (1080.0, pen.height);
    for band in margins.covered() {
        let area = band.area;
        let rect = (
            area.x * width,
            area.y * height,
            area.width * width,
            area.height * height,
        );
        pen.boxed(rect, 0.0, TINT);
    }
    let clear = margins.clear();
    let rect = (
        clear.x * width,
        clear.y * height,
        clear.width * width,
        clear.height * height,
    );
    pen.outline(rect, 0.0, (EDGE, 4.0));
}

/// The phone's own bar: the time at the left, signal, wifi and battery at the
/// right.
pub(super) fn status(pen: &mut Pen<'_>) {
    pen.text("9:41", (60.0, 62.0), 36.0, (true, WHITE));
    for (name, x) in [("signal", 890.0), ("wifi", 950.0), ("battery-full", 1015.0)] {
        pen.icon(name, (x, 50.0), 42.0, WHITE);
    }
}

/// A column of actions centred on `x`, from `top` down: each a symbol and,
/// under it, its count or name.
pub(super) fn rail(pen: &mut Pen<'_>, x: f32, top: f32, actions: &[(&str, &str)]) -> f32 {
    let mut y = top;
    for (name, label) in actions {
        pen.icon(name, (x, y), 68.0, WHITE);
        if !label.is_empty() {
            pen.centred(label, (x, y + 76.0), 28.0, (true, WHITE));
        }
        y += 150.0;
    }
    y
}

/// A placeholder profile picture at `centre`, ringed.
pub(super) fn avatar(pen: &mut Pen<'_>, centre: (f32, f32), radius: f32) {
    pen.disc(centre, radius, AVATAR);
    pen.ring(centre, radius, (WHITE, 3.0));
}

/// The app's bar along the foot of the screen: its symbols spread evenly,
/// each with a label under it when it has one.
pub(super) fn navigation(pen: &mut Pen<'_>, items: &[(&str, &str)]) {
    let top = pen.up(NAV);
    pen.boxed((0.0, top, 1080.0, NAV), 0.0, INK);
    let step = 1080.0 / items.len() as f32;
    for (at, (name, label)) in items.iter().enumerate() {
        let x = step * (at as f32 + 0.5);
        let y = if label.is_empty() {
            top + 70.0
        } else {
            top + 55.0
        };
        pen.icon(name, (x, y), 58.0, WHITE);
        if !label.is_empty() {
            pen.centred(label, (x, top + 118.0), 24.0, (false, SOFT));
        }
    }
}

/// A full-width call to action: a translucent bar with its words at the left
/// and a chevron at the right, its top `from_bottom` up from the foot.
pub(super) fn call_to_action(pen: &mut Pen<'_>, from_bottom: f32, fill: Rgba) {
    let top = pen.up(from_bottom);
    pen.boxed((32.0, top, 1016.0, 88.0), 18.0, fill);
    pen.text("Learn more", (68.0, top + 58.0), 38.0, (true, WHITE));
    pen.icon("chevron-right", (1000.0, top + 44.0), 44.0, WHITE);
}

/// A pill-shaped button whose left edge is `x`, centred on the line `y`:
/// filled, or outlined in white. Hands back where it ends.
pub(super) fn button(pen: &mut Pen<'_>, label: &str, (x, y): (f32, f32), filled: bool) -> f32 {
    let size = 32.0;
    let width = pen.width_of(label, size, true) + 48.0;
    let rect = (x, y - 30.0, width, 60.0);
    if filled {
        pen.boxed(rect, 30.0, WHITE);
        pen.text(label, (x + 24.0, y + 11.0), size, (true, INK));
    } else {
        pen.outline(rect, 14.0, (WHITE, 3.0));
        pen.text(label, (x + 24.0, y + 11.0), size, (true, WHITE));
    }
    x + width
}
