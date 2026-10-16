//! The palette, held to the web's tokens (#1228).
//!
//! The app follows the web's visuals: the web decides, the app mirrors it. The
//! values in [`palette`](super::palette) are hand-copied from
//! `web/src/index.css`, so this test reads that file, converts each token and
//! compares it with its counterpart. A colour changed on the web fails here
//! until the app matches.
//!
//! The asset-kind hues are not in the table: the web draws them with Tailwind
//! classes (`web/src/editor/assets/kinds.ts`), not tokens, until #1210.

mod oklch;

use egui::Color32;
use oklch::Oklch;

use super::palette::{DARK, LIGHT};

const CSS: &str = include_str!("../../../../web/src/index.css");

/// Each web token and the app colour that carries it, per theme.
fn pairing() -> [(&'static str, Color32, Color32); 9] {
    [
        // (token, light, dark)
        ("--background", LIGHT.background, DARK.background),
        ("--foreground", LIGHT.foreground, DARK.foreground),
        ("--card", LIGHT.card, DARK.card),
        ("--muted", LIGHT.muted, DARK.muted),
        (
            "--muted-foreground",
            LIGHT.muted_foreground,
            DARK.muted_foreground,
        ),
        ("--border", LIGHT.border, DARK.border),
        ("--input", LIGHT.input, DARK.input),
        ("--ring", LIGHT.ring, DARK.ring),
        ("--primary", LIGHT.primary, DARK.primary),
    ]
}

/// The declarations inside the rule that opens with `selector`.
fn block(selector: &str) -> &'static str {
    let from = CSS.find(&format!("\n{selector} {{")).expect(selector);
    let body = &CSS[from..];
    &body[..body.find('}').expect("an unclosed rule")]
}

/// The token's value in a block, as written.
fn token(block: &str, name: &str) -> Oklch {
    let line = block
        .lines()
        .find_map(|l| l.trim().strip_prefix(&format!("{name}:")))
        .unwrap_or_else(|| panic!("{name} is not declared"));
    let value = line.trim().trim_end_matches(';');
    Oklch::parse(value).unwrap_or_else(|| panic!("{name}: {value} is not an oklch"))
}

fn assert_close(what: &str, app: Color32, web: [f64; 3]) {
    let app = [app.r(), app.g(), app.b()];
    for (a, w) in app.iter().zip(web) {
        assert!(
            (f64::from(*a) - w).abs() <= 1.0 + 1e-9,
            "{what}: the app has {app:?}, the web {web:.1?}"
        );
    }
}

/// Every paired token, in both themes, within one step per channel.
#[test]
fn the_palette_is_the_webs() {
    for (name, theme, side) in [("light", ":root", 0), ("dark", ".dark", 1)] {
        let css = block(theme);
        let ground = token(css, "--background").over([0.0; 3]);
        for (token_name, light, dark) in pairing() {
            let app = [light, dark][side];
            assert_close(
                &format!("{token_name} ({name})"),
                app,
                token(css, token_name).over(ground),
            );
        }
    }
}

/// The destructive red, the warning amber and the playhead have tokens too.
#[test]
fn the_signal_colours_are_the_webs() {
    let (light, dark) = (block(":root"), block(".dark"));
    let white = [255.0; 3];
    assert_close(
        "--destructive (light)",
        LIGHT.destructive,
        token(light, "--destructive").over(white),
    );
    assert_close(
        "--destructive (dark)",
        DARK.destructive,
        token(dark, "--destructive").over(white),
    );
    assert_close(
        "--warning (light)",
        LIGHT.warning,
        token(light, "--warning").over(white),
    );
    assert_close(
        "--warning (dark)",
        DARK.warning,
        token(dark, "--warning").over(white),
    );
    assert_close(
        "--playhead (light)",
        LIGHT.playhead,
        token(light, "--playhead").over(white),
    );
    assert_close(
        "--playhead (dark)",
        DARK.playhead,
        token(dark, "--playhead").over(white),
    );
}
