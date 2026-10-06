//! An icon written back out as an SVG document, for a browser to draw.
//!
//! Pages are where new graphics go (CLAUDE.md, *the native graphics are
//! frozen*), and a page that wants a symbol is served this rather than having
//! one drawn from memory (#838). It is written from the same contours
//! [`super::draw`] paints, so the page and the `icon` asset draw one drawing,
//! not two copies of it that a re-vendoring could part.
//!
//! The document keeps Lucide's own vocabulary — a 24-unit `viewBox`,
//! `stroke="currentColor"`, round caps and joins, width 2 — so a page colours
//! and sizes it exactly as it would upstream's file: inline it and set
//! `color`, or use it as a CSS mask. The few filled dots are a second path
//! filled with that same `currentColor`.

use std::fmt::Write;

use super::draw::{Step, steps};
use super::{Icon, STROKE, VIEWBOX};

/// The document, or `None` for contours that do not decode.
pub(super) fn svg(icon: &Icon) -> Option<String> {
    let stroked = data(icon.stroked)?;
    let filled = data(icon.filled)?;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{VIEWBOX}\" height=\"{VIEWBOX}\" \
         viewBox=\"0 0 {VIEWBOX} {VIEWBOX}\" fill=\"none\" stroke=\"currentColor\" \
         stroke-width=\"{STROKE}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">"
    );
    if !stroked.is_empty() {
        let _ = write!(out, "<path d=\"{stroked}\"/>");
    }
    if !filled.is_empty() {
        let _ = write!(
            out,
            "<path d=\"{filled}\" fill=\"currentColor\" stroke=\"none\"/>"
        );
    }
    out.push_str("</svg>\n");
    Some(out)
}

/// One contour list as a path's `d` attribute, empty when it holds nothing.
fn data(commands: &[u8]) -> Option<String> {
    let mut d = String::new();
    for step in steps(commands)? {
        let (letter, points): (char, &[(f32, f32)]) = match &step {
            Step::Move(to) => ('M', std::slice::from_ref(to)),
            Step::Line(to) => ('L', std::slice::from_ref(to)),
            Step::Cubic(first, second, to) => ('C', &[*first, *second, *to]),
            Step::Quad(control, to) => ('Q', &[*control, *to]),
            Step::Close => ('Z', &[]),
        };
        d.push(letter);
        let numbers: Vec<String> = points
            .iter()
            .flat_map(|&(x, y)| [number(x), number(y)])
            .collect();
        d.push_str(&numbers.join(" "));
    }
    Some(d)
}

/// A coordinate to three decimals, trailing zeros dropped — a thousandth of a
/// 24-unit square is far below a pixel at any size a page draws one, and the
/// f32s the blob holds would otherwise print nine digits of noise each.
fn number(value: f32) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "-0" | "" => "0".to_owned(),
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn a_coordinate_is_written_short() {
        assert_eq!(number(12.0), "12");
        assert_eq!(number(4.5), "4.5");
        assert_eq!(number(1.234_567), "1.235");
        assert_eq!(number(-0.000_1), "0");
        assert_eq!(number(-3.25), "-3.25");
    }
}
