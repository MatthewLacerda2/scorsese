//! `oklch(L C H / A)` as the sRGB a browser paints, for holding the palette to
//! the web's tokens. By hand, to add no dependency: the standard OKLCH → OKLab
//! → linear sRGB → gamma path.

/// A parsed `oklch(…)`: lightness, chroma, hue in degrees, alpha 0..=1.
pub(super) struct Oklch {
    l: f64,
    c: f64,
    h: f64,
    alpha: f64,
}

impl Oklch {
    /// Reads `oklch(0.704 0.191 22.216)` and `oklch(1 0 0 / 10%)`.
    pub(super) fn parse(text: &str) -> Option<Self> {
        let inner = text.trim().strip_prefix("oklch(")?.strip_suffix(')')?;
        let (colour, alpha) = match inner.split_once('/') {
            Some((colour, alpha)) => {
                let percent = alpha.trim().strip_suffix('%')?.parse::<f64>().ok()?;
                (colour, percent / 100.0)
            }
            None => (inner, 1.0),
        };
        let mut parts = colour.split_whitespace().map(|p| p.parse::<f64>().ok());
        Some(Self {
            l: parts.next()??,
            c: parts.next()??,
            h: parts.next()??,
            alpha,
        })
    }

    /// The colour over `ground` (an sRGB triple), as 8-bit sRGB: what a browser
    /// composites, in the same gamma-encoded space egui does.
    pub(super) fn over(&self, ground: [f64; 3]) -> [f64; 3] {
        let own = self.srgb();
        [0, 1, 2].map(|i| own[i] * self.alpha + ground[i] * (1.0 - self.alpha))
    }

    /// Gamma-encoded sRGB, 0..=255, unrounded.
    fn srgb(&self) -> [f64; 3] {
        let (a, b) = (
            self.c * self.h.to_radians().cos(),
            self.c * self.h.to_radians().sin(),
        );
        let l = (self.l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
        let m = (self.l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
        let s = (self.l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
        let linear = [
            4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
            -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
            -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s,
        ];
        linear.map(|v| {
            let v = v.clamp(0.0, 1.0);
            let encoded = if v <= 0.003_130_8 {
                12.92 * v
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            };
            encoded * 255.0
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// White and black are the ends, whatever the conversion does between.
    #[test]
    fn white_and_black_convert_to_themselves() {
        let white = Oklch::parse("oklch(1 0 0)").unwrap().over([0.0; 3]);
        let black = Oklch::parse("oklch(0 0 0)").unwrap().over([0.0; 3]);
        assert!(white.iter().all(|v| (v - 255.0).abs() < 0.5), "{white:?}");
        assert!(black.iter().all(|v| v.abs() < 0.5), "{black:?}");
    }

    /// A translucent colour is mixed with what is under it.
    #[test]
    fn alpha_is_mixed_over_the_ground() {
        let half = Oklch::parse("oklch(1 0 0 / 50%)").unwrap().over([0.0; 3]);
        assert!(half.iter().all(|v| (v - 127.5).abs() < 0.5), "{half:?}");
    }

    /// Text that is not an `oklch(…)` is refused rather than guessed at.
    #[test]
    fn anything_else_does_not_parse() {
        assert!(Oklch::parse("#fff").is_none());
        assert!(Oklch::parse("oklch(1 0)").is_none());
    }
}
