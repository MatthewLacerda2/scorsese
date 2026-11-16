//! The part of an upright frame a feed's own interface covers (#969): its
//! caption, profile row, buttons and call to action, laid over the picture by
//! the platform, where text placed by the video is unreadable.
//!
//! Every figure is copied from the vendor's own page, with the day it was
//! read; a placement whose vendor publishes none carries no number, only what
//! the vendor does say. The written guide (`docs/styles.md`, *Safe zones*) and
//! a placement's brief state these values, and the interface overlay drawn
//! over a render (#1010) reads them from here.

use super::Platform;

#[cfg(test)]
mod tests;

/// What a placement's vendor publishes about the frame its interface covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafeZone {
    /// The margins to keep text out of, when the vendor publishes them.
    pub margins: Option<Margins>,
    /// Where the figures, or their absence, were read.
    pub source: Source,
}

/// A vendor page, and what it said when it was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    /// The vendor's own page.
    pub page: &'static str,
    /// The day it was read, `YYYY-MM-DD`.
    pub read: &'static str,
    /// What the page says, in a sentence, for a placement with no figures
    /// too.
    pub says: &'static str,
}

/// How much of each edge of the frame to keep free, as fractions of its
/// height (top, bottom) and width (left, right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    /// From the top edge, as a fraction of the height.
    pub top: f32,
    /// From the bottom edge, as a fraction of the height.
    pub bottom: f32,
    /// From the left edge, as a fraction of the width.
    pub left: f32,
    /// From the right edge, as a fraction of the width.
    pub right: f32,
    /// What the vendor says covers them, in its own words.
    pub covered_by: &'static str,
}

/// An edge of the frame, and the band along it a margin covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    /// The band across the top.
    Top,
    /// The band across the bottom.
    Bottom,
    /// The band down the left, between the top and bottom bands.
    Left,
    /// The band down the right, between the top and bottom bands.
    Right,
}

/// One covered band: which edge, and where it lies in the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Covered {
    /// The edge it runs along.
    pub edge: Edge,
    /// Its rectangle, in fractions of the frame.
    pub area: Area,
}

/// A rectangle in fractions of the frame, from its top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    /// Its left edge, as a fraction of the width.
    pub x: f32,
    /// Its top edge, as a fraction of the height.
    pub y: f32,
    /// Its width, as a fraction of the frame's.
    pub width: f32,
    /// Its height, as a fraction of the frame's.
    pub height: f32,
}

impl Area {
    /// Whether the two share any part of the frame, edges aside.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }
}

impl Margins {
    /// The part of the frame left clear for text.
    pub fn clear(&self) -> Area {
        Area {
            x: self.left,
            y: self.top,
            width: 1.0 - self.left - self.right,
            height: 1.0 - self.top - self.bottom,
        }
    }

    /// The four covered bands, top and bottom across the whole width, the
    /// sides between them, so they tile the frame with [`Self::clear`].
    pub fn covered(&self) -> [Covered; 4] {
        let middle = 1.0 - self.top - self.bottom;
        let band = |edge, x, y, width, height| Covered {
            edge,
            area: Area {
                x,
                y,
                width,
                height,
            },
        };
        [
            band(Edge::Top, 0.0, 0.0, 1.0, self.top),
            band(Edge::Bottom, 0.0, 1.0 - self.bottom, 1.0, self.bottom),
            band(Edge::Left, 0.0, self.top, self.left, middle),
            band(Edge::Right, 1.0 - self.right, self.top, self.right, middle),
        ]
    }

    /// The clear area in a `width`×`height` frame, as `(left, top, right,
    /// bottom)` pixels, rounded inward so a box inside them is inside it.
    pub fn clear_pixels(&self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let (w, h) = (f64::from(width), f64::from(height));
        let px = |value: f64| value as u32;
        (
            px((w * f64::from(self.left)).ceil()),
            px((h * f64::from(self.top)).ceil()),
            px((w * (1.0 - f64::from(self.right))).floor()),
            px((h * (1.0 - f64::from(self.bottom))).floor()),
        )
    }
}

/// What Meta's Ads Guide asks of an upright ad, for Reels and Stories alike.
const META_AD: Margins = Margins {
    top: 0.14,
    bottom: 0.35,
    left: 0.06,
    right: 0.06,
    covered_by: "cropped, covered by the profile icon or call-to-action, or placed too \
                 close to the edges of devices with screens taller than a 9:16 aspect ratio",
};

/// The day every figure here was read from its vendor's page.
const READ: &str = "2026-11-16";

/// TikTok's ad specification: the same page for an ad and a post.
const TIKTOK: Source = Source {
    page: "https://ads.tiktok.com/help/article/tiktok-auction-in-feed-ads",
    read: READ,
    says: "TikTok publishes its safe zone only as downloadable files, no figures in \
           text, and says its size depends on the caption's length and any add-ons; \
           nothing is published for a post.",
};

/// Google's video ad specification, whose only upright figures are for ads.
const GOOGLE_ADS: &str = "https://support.google.com/google-ads/answer/13547298";

impl Platform {
    /// What this placement's vendor publishes about the frame its interface
    /// covers.
    pub const fn safe_zone(self) -> SafeZone {
        let (margins, source) = match self {
            Self::Youtube => (
                None,
                Source {
                    page: GOOGLE_ADS,
                    read: READ,
                    says: "YouTube publishes no safe zone for a landscape video; Google's \
                           landscape figures are for ads, which scorsese has no placement for.",
                },
            ),
            Self::YoutubeShorts => (
                None,
                Source {
                    page: GOOGLE_ADS,
                    read: READ,
                    says: "YouTube publishes no figures for a posted Short (its editor draws \
                           guides as text is dragged); for upright ads, which run in the \
                           Shorts feed under the same buttons, Google's template keeps 288 px \
                           at the top, 672 at the bottom, 48 at the left and 192 at the right \
                           of 1080x1920 clear.",
                },
            ),
            Self::InstagramReels => (
                None,
                Source {
                    page: "https://www.facebook.com/business/ads-guide/update/video/instagram-reels",
                    read: READ,
                    says: "Meta publishes figures for Reels ads only, none for a posted Reel; \
                           the ad's are the nearest guide.",
                },
            ),
            Self::InstagramReelsAd => (
                Some(META_AD),
                Source {
                    page: "https://www.facebook.com/business/ads-guide/update/video/instagram-reels",
                    read: READ,
                    says: "Meta's Ads Guide: leave at least 14% of the top, 35% of the bottom \
                           and 6% on each side free from text, logos or other key elements.",
                },
            ),
            Self::InstagramStoriesAd => (
                Some(META_AD),
                Source {
                    page: "https://www.facebook.com/business/ads-guide/update/video/instagram-story",
                    read: READ,
                    says: "Meta's Ads Guide: leave roughly 14% of the top, 35% of the bottom \
                           and 6% on each side free from text, logos or other key elements.",
                },
            ),
            Self::Tiktok | Self::TiktokAd => (None, TIKTOK),
        };
        SafeZone { margins, source }
    }
}
