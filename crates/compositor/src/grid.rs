//! A ruler drawn over a finished frame, in the units the document takes.
//!
//! Every picture this project hands back answers *what is there*, and nothing
//! answered *where*. That gap is paid for in round trips: a crop rectangle or a
//! title's `transform.position.y` is guessed, rendered, looked at, and guessed
//! again, when a ruler on the picture would have shown the number the first
//! time.
//!
//! **Fractions, never pixels.** `crop` is a fraction of its source and
//! `transform.position.*` is a fraction of the raster, so a grid labelled in
//! pixels would be a second unit to convert out of — and the conversion would
//! change with the raster the picture happened to be made at. What is read off
//! the picture here is what gets typed into the document.
//!
//! Origin is the top-left corner, `x` rightwards and `y` downwards, because
//! that is where `crop` measures from.
//!
//! This is drawn **over** a finished frame rather than composited into one: it
//! is furniture for looking at a picture, not part of any edit, and nothing
//! that renders a file ever calls it.

use scorsese_core::Rgba;

use crate::frame::{Frame, Resolution};
use crate::paint;
use crate::text::{self, Font};

/// How many divisions the frame is cut into: a line every `0.1`.
const DIVISIONS: u32 = 10;

/// Which of those lines is drawn heavy — the middle one, `0.5`.
const HEAVY: u32 = DIVISIONS / 2;

/// The line itself: white, and not quite opaque, so what is underneath still
/// reads through the rule.
const RULE: Rgba = Rgba::new(0xff, 0xff, 0xff, 0xdd);

/// The dark edging drawn under every rule and behind every label.
///
/// A white line vanishes on a white shot and a black one vanishes on a night
/// exterior; a light mark with a dark edge survives both, which is the whole
/// reason there are two passes here rather than one.
const EDGE: Rgba = Rgba::new(0x00, 0x00, 0x00, 0xaa);

/// Em size of a label, as a fraction of the frame's shorter side.
const LABEL_SIZE: f64 = 0.045;

/// The smallest a label may be, in pixels.
///
/// A contact sheet's cell is a few hundred pixels tall, and a fraction alone
/// would put its labels at eight pixels — drawn, delivered, and unreadable,
/// which is the only way this feature can fail.
const LABEL_FLOOR: f64 = 13.0;

/// Draws the coordinate grid over `frame`.
///
/// The face is always the shipped sans, deliberately: this is furniture rather
/// than typography, and a project's own font is a choice about the video, not
/// about the ruler someone measures it with.
pub fn draw(frame: &mut Frame) {
    let resolution = frame.resolution();
    let unit = (f64::from(resolution.width().min(resolution.height())) / 540.0).max(1.0) as f32;

    // The edging first and the rule over it, so every line comes out as a pale
    // core inside a dark outline whatever it is drawn across.
    paint::fill_rects(frame, &rules(resolution, unit * 3.0, unit * 5.0), EDGE);
    paint::fill_rects(frame, &rules(resolution, unit, unit * 3.0), RULE);

    let size = label_size(resolution);
    let font = Font::sans();
    let labels = labels(resolution, font, size);
    let shadow = (size * 0.09).max(1.0);
    let shifted: Vec<(&str, (f32, f32))> = labels
        .iter()
        .map(|(text, (x, y))| (text.as_str(), (x + shadow, y + shadow)))
        .collect();
    text::draw_runs(frame, font, size, EDGE, &shifted);
    text::draw_runs(frame, font, size, Rgba::WHITE, &labels);
}

/// Every gridline as an `(x, y, width, height)` rectangle in pixels, `minor`
/// thick except the middle one, which is `major`.
///
/// Each line is centred on the fraction it marks and then held inside the
/// frame, so the `0.0` and `1.0` rules read as a full line rather than as the
/// half of one that happened to fall on the raster.
fn rules(resolution: Resolution, minor: f32, major: f32) -> Vec<(f32, f32, f32, f32)> {
    let width = resolution.width() as f32;
    let height = resolution.height() as f32;
    let mut rects = Vec::with_capacity(2 * (DIVISIONS as usize + 1));
    for step in 0..=DIVISIONS {
        let fraction = step as f32 / DIVISIONS as f32;
        let thickness = if step == HEAVY { major } else { minor };
        let inset = |along: f32| (along * fraction - thickness / 2.0).clamp(0.0, along - thickness);
        rects.push((inset(width), 0.0, thickness, height));
        rects.push((0.0, inset(height), width, thickness));
    }
    rects
}

/// How big a label is set, in pixels of this frame.
fn label_size(resolution: Resolution) -> f32 {
    let short = f64::from(resolution.width().min(resolution.height()));
    (short * LABEL_SIZE).max(LABEL_FLOOR) as f32
}

/// What each label reads and where its pen starts — `x` along the top edge,
/// `y` down the left one.
///
/// Horizontal labels all share one row at the top and sit to the right of the
/// line they name, except the last, which turns around rather than falling off
/// the picture. Vertical labels sit **above** their line, every one of them:
/// below would put `0.9` and `1.0` a single line apart at the foot of the
/// frame, which is the one place two of these ever collide.
///
/// The vertical `0.0` is left out. The top-left corner is already spoken for by
/// the horizontal one, and two labels in one place read as neither.
fn labels(resolution: Resolution, font: &Font, size: f32) -> Vec<(String, (f32, f32))> {
    let width = resolution.width() as f32;
    let height = resolution.height() as f32;
    let gap = size * 0.3;
    let mut runs = Vec::with_capacity(2 * DIVISIONS as usize + 1);
    for step in 0..=DIVISIONS {
        let fraction = step as f32 / DIVISIONS as f32;
        let text = format!("{fraction:.1}");

        let at = width * fraction;
        let x = if step == DIVISIONS {
            at - gap - text::width(font, &text, size)
        } else {
            at + gap
        };
        runs.push((text.clone(), (x, size * 1.05)));

        if step > 0 {
            runs.push((text, (gap, height * fraction - gap)));
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use crate::frame::BYTES_PER_PIXEL;

    use super::*;

    /// 1080p, where a label is 48 pixels and every number below is far enough
    /// from its neighbour that a pixel of anti-aliasing cannot reach it.
    fn hd() -> Resolution {
        Resolution::new(1920, 1080).expect("a legal raster")
    }

    /// How far right the **ink** of `text` reaches, drawn from `x = 0`.
    ///
    /// Measured off a raster rather than asked of [`text::width`], because
    /// that is the function this is here to hold: a placement worked out from
    /// a broken measurement would look perfectly correct to a test that used
    /// the same broken measurement.
    fn ink_reaches(text: &str, size: f32) -> f32 {
        let mut frame = Frame::black(hd());
        frame.fill_transparent();
        let origin = (0.0, size * 1.5);
        text::draw_line(&mut frame, text, Font::sans(), size, Rgba::WHITE, origin);
        let width = hd().width() as usize;
        let right = frame
            .bytes()
            .chunks_exact(BYTES_PER_PIXEL)
            .enumerate()
            .filter(|(_, pixel)| pixel[3] > 0)
            .map(|(at, _)| at % width)
            .max()
            .expect("the label drew something");
        right as f32 + 1.0
    }

    #[test]
    fn the_last_label_turns_around_rather_than_falling_off_the_picture() {
        // `1.0` is the only label that has to know how wide it is. Every other
        // one starts at the line it names and runs rightwards; this one would
        // run off the picture doing that, so it is placed by its right edge
        // instead — which is the single call to `text::width` in the crate,
        // and what nothing was asserting.
        let resolution = hd();
        let size = label_size(resolution);
        let gap = size * 0.3;
        let (text, (x, _)) = labels(resolution, Font::sans(), size)
            .into_iter()
            .find(|(text, (_, y))| text == "1.0" && (*y - size * 1.05).abs() < 0.01)
            .expect("the ruler labels the right-hand edge of the frame");
        let reaches = ink_reaches(&text, size);
        let edge = resolution.width() as f32;
        assert!(
            x + reaches <= edge - gap,
            "`1.0` reaches {} on a frame {edge} wide, so it is not the gap \
             short of the edge it was placed at",
            x + reaches
        );
        assert!(
            x + reaches > edge - gap - size,
            "…and it is still against that edge rather than somewhere left of \
             it: {}",
            x + reaches
        );
    }
}
