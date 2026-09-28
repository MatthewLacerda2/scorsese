//! Previews that trade picture for speed: a quality, and the proxies it may
//! read (#542).
//!
//! Getting a sense of pacing needs the cut to play in real time, and the cost
//! of a preview frame is mostly two things: how many pixels are composited,
//! and how big the source that had to be decoded to get them was. A
//! [`Quality`] answers the first — full, half or quarter of the delivery
//! raster — and a [`Proxies`] set the second, with a small copy of each heavy
//! video made once and decoded instead of the original.
//!
//! **Both are a preview's business and never a render's.** A [`Preview`] is
//! handed to a [`crate::Renderer`] only by something showing a person their
//! edit: the desktop app's picture, the web app's preview video. `scorsese
//! render`, the MCP `render` tool and the web app's finished renders never
//! build one, so there is no setting anywhere that makes a delivered file read
//! a proxy — the only way to get one is to ask for a preview by name.
//!
//! **Every quality shows the same picture**, at fewer pixels. A title, a
//! shape, a position and a fitted clip are all fractions of the raster and
//! scale by themselves. The one thing measured in the source's own pixels is a
//! `native` clip, and a preview at half the raster halves it too, so what is on
//! screen is the full-size frame drawn smaller rather than a different framing.
//! The same rule is what lets a proxy stand in for a `native` source: it is
//! scaled to the size the original would have arrived at.
//!
//! Full quality reads originals always. It is the picture a render delivers,
//! pixel for pixel, which is what makes it worth having next to the fast ones.

mod proxy;

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use scorsese_core::Asset;

use crate::settings::Resolution;

pub use proxy::{PROXY_SHORT_SIDE, Proxies, ProxyError, file_name, folder, make, worth_making};

/// How much of the delivery raster a preview draws.
///
/// One setting that means the same in the desktop app and the web app: a
/// fraction of the **delivery** raster — 1920x1080 for a landscape film — so
/// half is 960x540 wherever it is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Quality {
    /// Every pixel, from the originals: the render's own picture.
    Full,
    /// Half of each side, a quarter of the pixels, proxies where there are any.
    #[default]
    Half,
    /// A quarter of each side, a sixteenth of the pixels, proxies too.
    Quarter,
}

impl Quality {
    /// Every quality, best first — the order a control lists them in.
    pub const ALL: [Self; 3] = [Self::Full, Self::Half, Self::Quarter];

    /// The name a request or a setting spells it with: `full`, `half`,
    /// `quarter`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Half => "half",
            Self::Quarter => "quarter",
        }
    }

    /// What a control says: `Full`, `1/2`, `1/4`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Half => "1/2",
            Self::Quarter => "1/4",
        }
    }

    /// How many times smaller each side is than the delivery raster.
    pub const fn divisor(self) -> u32 {
        match self {
            Self::Full => 1,
            Self::Half => 2,
            Self::Quarter => 4,
        }
    }

    /// Whether a preview at this quality reads proxies where they exist. Full
    /// never does: it is the render's picture, and a proxy is not.
    pub const fn uses_proxies(self) -> bool {
        !matches!(self, Self::Full)
    }

    /// The raster a preview of a film delivered at `full` is drawn at.
    ///
    /// Each side divided and kept even, which is what the encoder a preview
    /// video goes through requires — so a 1080-tall film at a quarter is
    /// 270 tall, and an odd quotient rounds down a pixel rather than being
    /// refused.
    pub fn raster(self, full: Resolution) -> Resolution {
        let side = |pixels: u32| ((pixels / self.divisor()) & !1).max(2);
        Resolution::new(side(full.width()), side(full.height()))
            .expect("an even side of at least two pixels is a legal raster")
    }

    /// A size in the source's own pixels, scaled as this quality scales the
    /// raster — what a `native` clip arrives at, at least a pixel each way.
    pub(crate) fn shrink(self, size: Resolution) -> Resolution {
        let side = |pixels: u32| (pixels / self.divisor()).max(1);
        Resolution::source(side(size.width()), side(size.height())).unwrap_or(size)
    }
}

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A quality name that is not one of the three.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a preview quality: say full, half or quarter")]
pub struct QualityError(String);

impl FromStr for Quality {
    type Err = QualityError;

    fn from_str(text: &str) -> Result<Self, QualityError> {
        Self::ALL
            .into_iter()
            .find(|quality| quality.name() == text)
            .ok_or_else(|| QualityError(text.to_owned()))
    }
}

/// What a [`crate::Renderer`] is told when what it draws is a preview: the
/// quality, and the proxies it may read at that quality.
#[derive(Debug, Clone, Default)]
pub struct Preview {
    quality: Quality,
    proxies: Proxies,
}

impl Preview {
    /// A preview at `quality`, reading originals only.
    pub fn new(quality: Quality) -> Self {
        Self {
            quality,
            proxies: Proxies::default(),
        }
    }

    /// The same, reading `proxies` in place of their originals — when the
    /// quality is one that reads proxies at all.
    pub fn with_proxies(self, proxies: Proxies) -> Self {
        Self { proxies, ..self }
    }

    /// The quality it draws at.
    pub const fn quality(&self) -> Quality {
        self.quality
    }

    /// The file to decode in place of `asset`'s own, if there is one and this
    /// quality reads it.
    pub(crate) fn proxy_for(&self, asset: &Asset) -> Option<&Path> {
        if !self.quality.uses_proxies() {
            return None;
        }
        self.proxies.of(asset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_quality_is_a_fraction_of_the_delivery_raster_and_stays_even() {
        let hd = Resolution::HD;
        assert_eq!(Quality::Full.raster(hd).to_string(), "1920x1080");
        assert_eq!(Quality::Half.raster(hd).to_string(), "960x540");
        assert_eq!(Quality::Quarter.raster(hd).to_string(), "480x270");
        let portrait = Resolution::new(1080, 1920).expect("a legal raster");
        assert_eq!(Quality::Quarter.raster(portrait).to_string(), "270x480");
        let odd = Resolution::new(1284, 722).expect("a legal raster");
        assert_eq!(Quality::Quarter.raster(odd).to_string(), "320x180");
    }

    #[test]
    fn only_the_reduced_qualities_read_proxies() {
        assert!(!Quality::Full.uses_proxies());
        assert!(Quality::Half.uses_proxies());
        assert!(Quality::Quarter.uses_proxies());
    }

    #[test]
    fn a_quality_reads_back_from_its_own_name_and_nothing_else() {
        for quality in Quality::ALL {
            assert_eq!(quality.name().parse::<Quality>(), Ok(quality));
        }
        assert!("Half".parse::<Quality>().is_err());
        assert!("low".parse::<Quality>().is_err());
    }

    #[test]
    fn a_native_size_shrinks_with_the_raster_and_never_to_nothing() {
        let source = Resolution::source(1921, 3).expect("a size");
        assert_eq!(Quality::Half.shrink(source).to_string(), "960x1");
        assert_eq!(Quality::Full.shrink(source), source);
    }
}
