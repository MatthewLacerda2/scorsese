//! What a [`ChromaKey`] does to pixels: the screen taken out, and the colour it
//! bounced back taken off what is left.
//!
//! `scorsese-core` says what the four values *are*. This is where they acquire
//! arithmetic, in one place, next to the loop that runs it — the same split
//! [`crate::grade`] has.
//!
//! **Straight RGBA in, straight RGBA out.** A key is the one operation in this
//! crate that writes *alpha* rather than only colour, and it has to run in
//! straight alpha for the reason a grade does: the colour it is measuring is
//! the colour the camera recorded, not that colour already scaled by how much
//! of it survives.
//!
//! # The plane, and why it is not RGB
//!
//! A real green screen has **one chroma and many lumas**. It is lit unevenly,
//! it falls off toward the edges, the subject casts shadows on it — so a plain
//! distance in RGB keys the lit half of the screen and keeps the shadowed half,
//! which is a keyer that only works on flat synthetic backgrounds.
//!
//! So the distance is measured with the light divided out. Each pixel is
//! reduced to its **chromaticity** — the proportions of the three channels,
//! `r / (r+g+b)` and so on — which is exactly invariant under scaling all three
//! together. A pixel and the same pixel at half the exposure land on the same
//! point, so a screen lit from one side keys as one screen.
//!
//! That triangle is then drawn as an **equilateral** one, white at the origin
//! and each primary exactly `1.0` from it, so a distance means the same thing
//! in every direction. Written in the proportions directly it would not: the
//! obvious `(r, g)` plane puts green `0.745` from white and blue `0.471`, and a
//! tolerance would silently mean something different for a blue screen than for
//! a green one.
//!
//! **Below a floor, a pixel has no chromaticity and is kept.** Proportions of
//! nearly nothing are noise — one level either way swings them across the whole
//! plane — so a near-black pixel is left alone rather than keyed on a coin
//! flip. The failure that leaves is a shadow so deep it stays opaque, which is
//! a mark on the matte somebody can see; keying on noise instead speckles the
//! subject.
//!
//! # Spill, generalised past green
//!
//! The classic despill is one comparison — green above what red and blue
//! justify is bounce off the screen, so it is pulled back to what they justify.
//! Said that way it is about green, and it must not be: magenta is the better
//! screen for some sources and for generated stills in particular.
//!
//! Generalised, the statement is about the component **along whatever hue was
//! keyed**. The key colour becomes a weight per channel, and the pixel's level
//! along those weights is pulled down to the level along the channels the key
//! does not use. For a pure green key that is exactly `g → (r + b) / 2`; for
//! magenta it pulls red and blue down toward green; for the yellow-greens real
//! screens are painted it splits the difference in the proportion the key does.
//!
//! **Then the luma is put back.** Green carries 72% of Rec.709 luma, so pulling
//! it down leaves the edge it was pulled from visibly darker — a grey rim
//! replacing the green one, which is not an improvement. Scaling the despilled
//! pixel back to the luma it had restores the brightness while keeping the hue
//! the despill just corrected.
//!
//! **Both of those were settled on a rendered frame**, on a plate keyed against
//! `#00b140` with a half-covered strand of hair carrying heavy bounce. On that
//! strand, `(99, 168, 102)` at luma 148.6:
//!
//! | despill | the strand | luma |
//! | --- | --- | --- |
//! | none | `(99, 168, 102)` | 148.6 |
//! | clamped to the strongest other channel | `(99, 117, 83)` | 110.7 |
//! | blended to their average | `(99, 118, 84)` | 111.5 |
//! | and put back at its own luma | `(132, 157, 112)` | 148.4 |
//!
//! The last row is what ships, and the third column is why: a quarter of the
//! strand's brightness leaves with the green, which is a grey rim where there
//! was a green one. What separates the middle two is not that row but a
//! *mildly* spilled pixel — `(199, 201, 155)` on the body's lit edge, which the
//! average pulls to `(199, 194, 152)` and the hard clamp leaves exactly alone,
//! because the strongest other channel is always at least their average and on
//! a bright pixel it is far above it. So the clamp is the one that misses the
//! spill nobody would otherwise notice until it is next to a background.

use scorsese_core::{ChromaKey, Rgba};

use crate::frame::BYTES_PER_PIXEL;

/// Half of √3, which is where an equilateral triangle's other two vertices sit
/// once one of them is on the x-axis.
///
/// Written out rather than computed, because a `const` cannot call `sqrt` and a
/// `sqrt` at run time is a square root taken per layer to reach a number that
/// never changes.
const ROOT_THREE_HALVES: f64 = 0.866_025_403_784_438_6;

/// The sum of a pixel's three channels, below which it has no chromaticity
/// worth measuring.
///
/// Twelve levels out of 255 across all three channels — an average channel of
/// four. At that level a single level of noise moves the proportions by around
/// a tenth of the plane, which is comparable to a whole tolerance, so the
/// answer would be decided by the encoder rather than by the colour.
const DARK: f64 = 12.0 / 255.0;

/// Rec.709 luma weights — the same ones [`crate::grade`] greys a pixel with,
/// and here for the same reason: green carries most of the perceived
/// brightness, so a despill that pulls green down has to know how much light it
/// just took away.
const LUMA: (f64, f64, f64) = (0.2126, 0.7152, 0.0722);

/// A key resolved for one layer: everything that depends on the four values and
/// not on the pixel, worked out once.
///
/// Reused rather than reimplemented, the way [`crate::grain::Grain`] is:
/// anything else wanting to decide a pixel's alpha from its colour builds one of
/// these and calls [`Keyer::at`] rather than growing a second keyer that drifts
/// from this one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Keyer {
    /// Where the screen's colour sits in the equilateral chromaticity plane.
    screen: (f64, f64),
    /// Distance at or below which a pixel is entirely screen. Never negative.
    tolerance: f64,
    /// How far past the tolerance the ramp back to opaque runs. Never negative,
    /// and zero is a hard cutout.
    softness: f64,
    /// The despill, when it was asked for and the key colour admits of one.
    spill: Option<Spill>,
}

/// The despill's per-layer arithmetic: the key colour as a weight per channel,
/// and the two sums those weights divide by.
#[derive(Debug, Clone, Copy)]
struct Spill {
    /// The key colour scaled so its strongest channel is `1.0`. This is what
    /// makes the operation about the hue rather than about green.
    weights: [f64; 3],
    /// `Σ weights` — what the pixel's level *along* the key divides by.
    along: f64,
    /// `Σ (1 − weights)` — what the level along everything else divides by.
    across: f64,
}

impl Keyer {
    /// A keyer for `key`, or `None` when there is nothing to key against.
    ///
    /// The one refusal is a screen colour so dark it has no chromaticity: the
    /// distance would be measured from a point that is noise. A colour with no
    /// *hue* is refused a step earlier, by validation, because that is a luma
    /// key and a sentence about the document — see [`ChromaKey::is_keyable`].
    pub(crate) fn new(key: ChromaKey) -> Option<Self> {
        let screen = chromaticity(normalise(key.color))?;
        Some(Self {
            // Negative is not a tolerance and neither is a non-number, and both
            // leave by the same door zero does. `max` alone would let a NaN
            // through, since it prefers the other operand.
            tolerance: sane(key.tolerance),
            softness: sane(key.softness),
            screen,
            spill: key.spill.then(|| Spill::new(key.color)).flatten(),
        })
    }

    /// One pixel, straight RGBA in and straight RGBA out.
    ///
    /// The alpha it hands back is the pixel's own, scaled by how much of it
    /// survives the key — so a source that arrived with alpha of its own keeps
    /// it, and a key can only ever take opacity away.
    pub(crate) fn at(&self, pixel: [u8; 4]) -> [u8; 4] {
        let (r, g, b) = normalise(Rgba::new(pixel[0], pixel[1], pixel[2], pixel[3]));
        // A pixel with no chromaticity is kept whole, despill included: there
        // is no hue in it to have come off the screen.
        let Some(at) = chromaticity((r, g, b)) else {
            return pixel;
        };
        let (dx, dy) = (at.0 - self.screen.0, at.1 - self.screen.1);
        let distance = dx.hypot(dy);
        let kept = self.kept(distance);
        let (r, g, b) = match self.spill {
            Some(spill) => spill.at(r, g, b),
            None => (r, g, b),
        };
        [
            quantise(r),
            quantise(g),
            quantise(b),
            quantise(f64::from(pixel[3]) / 255.0 * kept),
        ]
    }

    /// How much of a pixel at `distance` from the screen colour survives: `0.0`
    /// entirely screen, `1.0` entirely subject, and a straight ramp between.
    ///
    /// Linear, because it is the neutral shape and the one anybody turning the
    /// two knobs can predict — the same reason a fade is linear until somebody
    /// says otherwise.
    fn kept(&self, distance: f64) -> f64 {
        if distance <= self.tolerance {
            return 0.0;
        }
        // A hard cutout is what zero softness means, and it is also the case
        // the division below cannot express.
        if self.softness <= 0.0 {
            return 1.0;
        }
        ((distance - self.tolerance) / self.softness).min(1.0)
    }
}

impl Spill {
    /// The despill for a screen of this colour, or `None` when there is no
    /// despill to do.
    ///
    /// `None` for black — nothing to take a proportion of — and for white,
    /// where every channel is along the key and there is nothing left for the
    /// pixel's level to be pulled back *to*. Validation refuses both as key
    /// colours, so this is the arithmetic refusing to divide by zero rather
    /// than a second opinion about them.
    fn new(color: Rgba) -> Option<Self> {
        let peak = f64::from(color.r.max(color.g).max(color.b)) / 255.0;
        if peak <= 0.0 {
            return None;
        }
        let (r, g, b) = normalise(color);
        let weights = [r / peak, g / peak, b / peak];
        let along = weights.iter().sum::<f64>();
        let across = weights.iter().map(|weight| 1.0 - weight).sum::<f64>();
        (across > 0.0).then_some(Self {
            weights,
            along,
            across,
        })
    }

    /// One pixel, despilled and put back at the luma it had.
    fn at(&self, r: f64, g: f64, b: f64) -> (f64, f64, f64) {
        let (mut along, mut across) = (0.0, 0.0);
        for (channel, weight) in [r, g, b].into_iter().zip(self.weights) {
            along += channel * weight;
            across += channel * (1.0 - weight);
        }
        // How much more of the key's own hue this pixel carries than the
        // channels the key does not use can account for. Positive is bounce off
        // the screen; zero or less is a colour the scene had.
        let excess = along / self.along - across / self.across;
        if excess <= 0.0 {
            return (r, g, b);
        }
        let pulled = [
            r - self.weights[0] * excess,
            g - self.weights[1] * excess,
            b - self.weights[2] * excess,
        ];
        let before = luma(r, g, b);
        let after = luma(pulled[0], pulled[1], pulled[2]);
        // Scaled rather than offset, so the hue the despill just corrected is
        // the hue that comes back brighter; adding grey would put a little of
        // the screen's colour back as a wash. A pixel with no light left in it
        // has no ratio to scale by and keeps what it has.
        let gain = if after > 0.0 { before / after } else { 1.0 };
        (pulled[0] * gain, pulled[1] * gain, pulled[2] * gain)
    }
}

/// Writes `source` into `out`, keyed, and hands back the result.
///
/// Returns `source` untouched when there is no key to apply, so the caller can
/// use the answer unconditionally — the same contract [`crate::blur::into`] and
/// [`crate::aberration::into`] have, and for one of the same two reasons: a
/// buffer whose length disagrees with the resolution it claims is refused a few
/// lines later, where every other malformed layer is.
pub(crate) fn into<'a>(
    out: &'a mut Vec<u8>,
    source: &'a [u8],
    key: Option<ChromaKey>,
) -> &'a [u8] {
    let Some(keyer) = key.and_then(Keyer::new) else {
        return source;
    };
    out.clear();
    out.reserve(source.len());
    for pixel in source.chunks_exact(BYTES_PER_PIXEL) {
        out.extend_from_slice(&keyer.at([pixel[0], pixel[1], pixel[2], pixel[3]]));
    }
    out.as_slice()
}

/// A colour as three channels in `0.0..=1.0`, which is what all the arithmetic
/// here runs on. Alpha is not one of them: a key reads the colour the camera
/// recorded.
fn normalise(color: Rgba) -> (f64, f64, f64) {
    (
        f64::from(color.r) / 255.0,
        f64::from(color.g) / 255.0,
        f64::from(color.b) / 255.0,
    )
}

/// Where a colour sits in the equilateral chromaticity plane: white at the
/// origin, each primary exactly `1.0` out, two primaries `√3` apart.
///
/// `None` below [`DARK`], where the proportions are noise rather than a colour.
fn chromaticity((r, g, b): (f64, f64, f64)) -> Option<(f64, f64)> {
    let sum = r + g + b;
    if sum < DARK {
        return None;
    }
    let (r, g, b) = (r / sum, g / sum, b / sum);
    // Red at (1, 0) and the other two at 120° from it: `(3r − 1) / 2` is `r`
    // measured from the white point and scaled so a primary lands on the unit
    // circle, and the difference of the other two carries the perpendicular.
    Some(((3.0 * r - 1.0) / 2.0, (g - b) * ROOT_THREE_HALVES))
}

/// Rec.709 luma, which is what "how bright is this pixel" means here.
fn luma(r: f64, g: f64, b: f64) -> f64 {
    LUMA.0 * r + LUMA.1 * g + LUMA.2 * b
}

/// A distance somebody wrote, with everything that is not one turned into
/// zero: a negative distance is not one, and neither is a NaN.
fn sane(value: f64) -> f64 {
    if value.is_nan() || value < 0.0 { 0.0 } else { value }
}

/// One channel back to a byte, clamped for [`crate::grade`]'s reason — the
/// despill's luma restoration can push a bright channel past white, and a
/// channel that wrapped would put a black speckle in the middle of it.
fn quantise(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The plane and the ramp, which nothing outside this file can reach.
///
/// Here rather than in `tests/keying/` for [`crate::aberration`]'s reason: that
/// is where the *effect* is asserted — a screen goes, an edge ramps, a rim
/// loses its colour — and an effect is satisfied by more than one arithmetic.
/// What these name are the numbers themselves.
#[cfg(test)]
mod tests {
    use super::*;

    /// The screen most of them are painted, and the one the docs quote.
    const SCREEN: Rgba = Rgba::opaque(0, 177, 64);

    fn at(color: Rgba) -> (f64, f64) {
        chromaticity(normalise(color)).expect("a colour with light in it")
    }

    fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    /// White at the origin and each primary exactly one out, which is the whole
    /// claim the plane makes and what makes a tolerance mean one thing in every
    /// direction. The obvious `(r, g)` plane fails this: green would be 0.745
    /// from white and blue 0.471.
    #[test]
    fn every_primary_is_exactly_one_from_white() {
        let white = at(Rgba::opaque(255, 255, 255));
        assert!(white.0.abs() < 1e-12 && white.1.abs() < 1e-12, "{white:?}");
        for primary in [
            Rgba::opaque(255, 0, 0),
            Rgba::opaque(0, 255, 0),
            Rgba::opaque(0, 0, 255),
        ] {
            let out = distance(at(primary), white);
            assert!((out - 1.0).abs() < 1e-12, "{primary} sits at {out}");
        }
        // And two primaries are √3 apart, which is the same statement made
        // where a wrong scale on one axis alone would still satisfy the first.
        let split = distance(at(Rgba::opaque(255, 0, 0)), at(Rgba::opaque(0, 255, 0)));
        assert!((split - 3.0_f64.sqrt()).abs() < 1e-12, "{split}");
    }

    /// The requirement the whole plane exists for: a screen lit unevenly is one
    /// chroma and many lumas, and only the chroma decides.
    ///
    /// The same point to the last rounding error, because dividing by the sum is
    /// what scaling all three channels leaves alone. A plane that merely put
    /// them *near* each other would drift apart on a screen lit twice as
    /// brightly at one end; this one cannot, and the test says so at a
    /// thousandth of the loosest tolerance anybody would write.
    ///
    /// The other half of the assertion is what makes it worth having: measured
    /// in RGB, the darkest of these sits 0.65 from the lit one — further than a
    /// primary is from white — so an RGB keyer takes the lit end of a screen
    /// and leaves the shadowed end standing.
    #[test]
    fn the_same_colour_at_any_exposure_is_the_same_point() {
        let lit = normalise(SCREEN);
        for shade in [0.68, 0.42, 0.12] {
            let dimmed = (lit.0 * shade, lit.1 * shade, lit.2 * shade);
            let (here, there) = (
                chromaticity(dimmed).expect("light in it"),
                chromaticity(lit).expect("light in it"),
            );
            assert!(distance(here, there) < 1e-12, "at {shade}: {here:?}");
        }
        let darkest = (lit.0 * 0.12, lit.1 * 0.12, lit.2 * 0.12);
        let in_rgb = ((lit.0 - darkest.0).powi(2)
            + (lit.1 - darkest.1).powi(2)
            + (lit.2 - darkest.2).powi(2))
        .sqrt();
        assert!(in_rgb > 0.6, "an RGB distance would be {in_rgb}");
        // And once quantised to the eight bits a decoder hands over, near
        // enough that a tolerance never notices: an eighth of the light is
        // eight levels of green, where a rounded level moves the proportions
        // more than the light does.
        let dimmed = Rgba::opaque(0, 74, 27);
        assert!(distance(at(dimmed), at(SCREEN)) < 0.005);
    }

    /// Below the floor there is no colour to measure, so the pixel is kept —
    /// and the boundary is the boundary, not a number beside it.
    #[test]
    fn a_pixel_with_no_light_in_it_has_no_chromaticity() {
        assert_eq!(chromaticity((0.0, 0.0, 0.0)), None);
        // DARK is a sum of three channels, so an even split of it sits exactly
        // on the boundary and is admitted; one level less is not.
        let third = DARK / 3.0;
        assert!(chromaticity((third, third, third)).is_some());
        let under = (DARK - 1.0 / 255.0) / 3.0;
        assert_eq!(chromaticity((under, under, under)), None);
        // And it is the whole pixel that survives, alpha included, rather than
        // being keyed on proportions that are noise.
        let keyer = Keyer::new(ChromaKey::new(SCREEN)).expect("a keyer");
        assert_eq!(keyer.at([1, 2, 1, 255]), [1, 2, 1, 255]);
    }

    /// The two thresholds every keyer has, at the two points that pin them.
    ///
    /// Both ends stated, because an assertion that a ramp exists is satisfied by
    /// a ramp running the other way, and each boundary is inclusive on the side
    /// its comparison says it is.
    #[test]
    fn the_ramp_runs_from_gone_at_the_tolerance_to_whole_past_the_softness() {
        let keyer = Keyer::new(ChromaKey {
            color: SCREEN,
            tolerance: 0.2,
            softness: 0.4,
            spill: false,
        })
        .expect("a keyer");
        assert_eq!(keyer.kept(0.0), 0.0, "the screen itself is gone");
        assert_eq!(keyer.kept(0.2), 0.0, "and so is the tolerance exactly");
        assert!((keyer.kept(0.4) - 0.5).abs() < 1e-12, "half way is half");
        // Within a rounding error of whole at the far end exactly, rather than
        // whole: `(0.6 − 0.2) / 0.4` is a hair under one in binary, and a byte
        // of alpha rounds it to 255 either way — `tests/keying/` is where the
        // claim about the byte is made.
        assert!((keyer.kept(0.6) - 1.0).abs() < 1e-12, "the far end is whole");
        assert_eq!(keyer.kept(9.0), 1.0, "and nothing past it is more");
        // A hair inside each end, so a boundary moved by an epsilon is caught
        // rather than landing on the same answer.
        assert!(keyer.kept(0.2001) > 0.0);
        assert!(keyer.kept(0.5999) < 1.0);
    }

    /// No softness is a hard cutout, which is the one case the ramp's division
    /// cannot express — and the case a `<= 0.0` mutated to `< 0.0` would send
    /// through it.
    #[test]
    fn no_softness_is_a_cutout_rather_than_a_division_by_zero() {
        let keyer = Keyer::new(ChromaKey {
            color: SCREEN,
            tolerance: 0.2,
            softness: 0.0,
            spill: false,
        })
        .expect("a keyer");
        assert_eq!(keyer.kept(0.2), 0.0);
        assert_eq!(keyer.kept(0.200_001), 1.0);
    }

    /// Nothing that is not a distance is one, and both leave by zero's door.
    #[test]
    fn a_negative_or_a_non_number_keys_only_an_exact_match() {
        let keyer = Keyer::new(ChromaKey {
            color: SCREEN,
            tolerance: -1.0,
            softness: f64::NAN,
            spill: false,
        })
        .expect("a keyer");
        assert_eq!(keyer.kept(0.0), 0.0, "the screen itself still goes");
        assert_eq!(keyer.kept(0.000_001), 1.0, "and nothing else does");
    }

    /// The despill is about the hue that was keyed and not about green, which
    /// is the whole reason the weights exist.
    #[test]
    fn the_despill_pulls_down_whichever_hue_was_keyed() {
        let green = Spill::new(Rgba::opaque(0, 255, 0)).expect("a despill");
        // A pure green key is exactly the classic `g → (r + b) / 2`.
        let (r, g, b) = green.at(0.4, 0.8, 0.2);
        assert!((g / (r + b) - 0.5).abs() < 1e-9, "({r}, {g}, {b})");
        // Magenta pulls red and blue toward green instead, which is the same
        // sentence about a different screen.
        let magenta = Spill::new(Rgba::opaque(255, 0, 255)).expect("a despill");
        let (r, g, b) = magenta.at(0.8, 0.2, 0.6);
        assert!((g / (r + b) - 0.5).abs() < 1e-9, "({r}, {g}, {b})");
    }

    /// And it puts the light back, which is what stops a despilled edge reading
    /// as a grey rim where there was a green one.
    #[test]
    fn a_despilled_pixel_keeps_the_brightness_it_had() {
        let spill = Spill::new(SCREEN).expect("a despill");
        let (before, after) = ((0.39, 0.66, 0.4), spill.at(0.39, 0.66, 0.4));
        assert!(after.1 < before.1, "the green came down: {after:?}");
        let (was, is) = (
            luma(before.0, before.1, before.2),
            luma(after.0, after.1, after.2),
        );
        assert!((was - is).abs() < 1e-9, "{was} became {is}");
    }

    /// A colour the scene had is a colour the scene keeps: nothing is pulled
    /// out of a pixel that carries no more of the key's hue than the rest of it
    /// accounts for.
    #[test]
    fn a_pixel_with_no_bounce_on_it_is_left_alone() {
        let spill = Spill::new(SCREEN).expect("a despill");
        for pixel in [(0.78, 0.63, 0.55), (1.0, 1.0, 1.0), (0.2, 0.2, 0.2)] {
            let out = spill.at(pixel.0, pixel.1, pixel.2);
            assert!(
                (out.0 - pixel.0).abs() < 1e-12
                    && (out.1 - pixel.1).abs() < 1e-12
                    && (out.2 - pixel.2).abs() < 1e-12,
                "{pixel:?} became {out:?}"
            );
        }
    }

    /// Nothing to do is no work at all, and a key nothing can be measured from
    /// is refused rather than measured from noise.
    #[test]
    fn nothing_to_key_against_is_no_work_at_all() {
        let source = vec![7; 4 * BYTES_PER_PIXEL];
        let mut out = Vec::new();
        assert_eq!(into(&mut out, &source, None), &source[..]);
        assert!(out.is_empty(), "no key allocates nothing");
        let black = Some(ChromaKey::new(Rgba::opaque(0, 0, 0)));
        assert_eq!(into(&mut out, &source, black), &source[..]);
        assert!(out.is_empty(), "and neither does a screen with no colour");
        assert!(Keyer::new(ChromaKey::new(Rgba::opaque(0, 0, 0))).is_none());
        // The despill has the other end of that: white is along the key in
        // every channel, so there is nothing left to pull a pixel back to.
        assert!(Spill::new(Rgba::opaque(255, 255, 255)).is_none());
        assert!(Spill::new(Rgba::opaque(0, 0, 0)).is_none());
    }
}
