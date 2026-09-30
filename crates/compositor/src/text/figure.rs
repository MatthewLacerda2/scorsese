//! Keeping a counting number's room while it counts.
//!
//! A figure that grows from `0` to `1.234` gains four characters on the way,
//! and a line centred on it would slide left by one of them every time it
//! gained one. So the figure is padded, on the left, to the width of the widest
//! one it will reach — with the two spaces Unicode defines for exactly this: a
//! **figure space** wherever the widest figure has a digit, and a
//! **punctuation space** wherever it has a separator. The digits then sit in
//! the columns they will finish in, the way an odometer's do.
//!
//! **What this relies on is the face.** A figure space is as wide as the face's
//! tabular digit, and the digits are set tabular ([`super::Figures`]) — so in a
//! face with `tnum` the padded line is exactly as wide at every value. A face
//! without it keeps its proportional digits: the padding still holds the line
//! to the right number of columns, but a `1` is narrower than an `8`, so the
//! line can still shift by a fraction of a digit as it counts. A face with no
//! figure space at all is given one by the shaper, as wide as its `0`.

use super::shape::{FIGURE_SPACE, PUNCTUATION_SPACE};

/// `figure`, padded on the left to as many columns as `widest` has.
///
/// Right-aligned because figures are: the units stay put and the thousands
/// arrive at the front. A figure already as long as `widest`, or longer, comes
/// back as it is.
pub fn padded(figure: &str, widest: &str) -> String {
    let short = widest
        .chars()
        .count()
        .saturating_sub(figure.chars().count());
    let mut out: String = widest
        .chars()
        .take(short)
        .map(|column| {
            if column.is_ascii_digit() {
                FIGURE_SPACE
            } else {
                PUNCTUATION_SPACE
            }
        })
        .collect();
    out.push_str(figure);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{Figures, Font};

    #[test]
    fn a_short_figure_keeps_the_widest_one_s_columns() {
        assert_eq!(padded("56", "1.234"), "\u{2007}\u{2008}\u{2007}56");
        assert_eq!(padded("1.234", "1.234"), "1.234");
        assert_eq!(padded("12345", "99"), "12345");
    }

    /// The whole of the no-jitter claim, measured: in the default sans, which
    /// has `tnum`, a figure padded to the widest one sets exactly as wide as
    /// the widest one — and without tabular figures it does not, which is why
    /// they are asked for.
    #[test]
    fn a_padded_figure_in_tabular_figures_is_as_wide_as_the_widest() {
        let font = Font::sans();
        let tabular = font.faces(40.0).with(Figures::Tabular);
        let widest = "8.888";
        for figure in ["1", "17", "111", "1.111"] {
            let width = tabular.shape(&padded(figure, widest)).width;
            assert!(
                (width - tabular.shape(widest).width).abs() < 0.01,
                "`{figure}` padded sets {width}"
            );
        }
        let proportional = font.faces(40.0);
        assert_ne!(
            proportional.shape("111").width,
            proportional.shape("888").width,
            "Inter's default figures are proportional, so the test above is a claim"
        );
    }
}
