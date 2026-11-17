//! A platform's interface, drawn over a finished frame to preview a video as
//! it will look in the feed (#1010).
//!
//! **A per-render choice, never project content.** A mock interface written
//! into `project.json` is one forgotten deletion away from shipping inside a
//! real ad, so the overlay lives in [`crate::RenderSettings`] beside the other
//! things a render decides, is laid over each frame after the compositor has
//! finished it, and is named by the same [`Platform`] id a render is sized by.
//!
//! **Drawn here, in Rust, rather than shipped as a page or a picture.** A page
//! needs a browser and does not capture on Windows; a pre-rendered PNG is one
//! raster scaled to every other, needs a decoder this crate does not have, and
//! is a binary regenerated outside the build. Drawn from the compositor's own
//! marks, Lucide symbols and shipped type, it costs no process, draws the same
//! on every machine, and is sharp at a render's size and at a sheet cell's.
//!
//! **Two kinds of truth, kept apart.** The controls (status bar, profile row,
//! action rail, caption, call to action, navigation) are a generic,
//! recognisable approximation: no vendor publishes where they sit, and no
//! logo or trademark is drawn. The safe-zone tint is the vendor's own figures
//! ([`Platform::safe_zone`]) and nothing else: a placement whose vendor
//! publishes none gets no tint, and [`Overlay::said`] says so.

mod chrome;
mod instagram;
mod pen;
mod shorts;
#[cfg(test)]
mod tests;
mod tiktok;

use std::fmt;

use scorsese_compositor::{Frame, Resolution};
use scorsese_core::style::Platform;

use pen::Pen;

/// A platform's interface to lay over a render, and whether its safe zones
/// are tinted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overlay {
    platform: Platform,
    safe_zones: bool,
}

/// Why an overlay cannot be drawn as asked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OverlayError {
    /// A placement watched in a player rather than a feed.
    #[error(
        "overlay: {0} is watched in a landscape player whose controls fade away, so there is \
         no feed interface to draw; one of {list}",
        list = upright()
    )]
    NoInterface(Platform),
    /// A frame a phone's feed never shows.
    #[error(
        "overlay: a feed's interface is drawn over a 9:16 phone screen and this picture is \
         {0}; give it a 9:16 frame (a 9:16 resolution, or the platform's own size)"
    )]
    NotUpright(Resolution),
    /// Safe zones asked for with no interface to tint.
    #[error("safe_zones: tints the overlay's safe zones, so it needs an overlay named")]
    ZonesWithoutOverlay,
}

impl Overlay {
    /// The interface of `platform`, with its safe zones tinted when
    /// `safe_zones` asks and its vendor publishes them.
    pub fn new(platform: Platform, safe_zones: bool) -> Result<Self, OverlayError> {
        if platform == Platform::Youtube {
            return Err(OverlayError::NoInterface(platform));
        }
        Ok(Self {
            platform,
            safe_zones,
        })
    }

    /// The placement whose interface this is.
    pub fn platform(self) -> Platform {
        self.platform
    }

    /// Whether the safe zones were asked to be tinted.
    pub fn safe_zones(self) -> bool {
        self.safe_zones
    }

    /// Whether this frame can carry it: a phone's 9:16, give or take the
    /// rounding of a small raster. The layouts place the header from the top
    /// and the controls from the foot, so a shorter frame (4:5, 3:4) would
    /// lay one over the other.
    pub fn fits(self, resolution: Resolution) -> Result<(), OverlayError> {
        let shape = f64::from(resolution.width()) / f64::from(resolution.height());
        if (0.53..=0.6).contains(&shape) {
            Ok(())
        } else {
            Err(OverlayError::NotUpright(resolution))
        }
    }

    /// Whether a tint is drawn: asked for, and the vendor publishes figures.
    pub fn tinted(self) -> bool {
        self.safe_zones && self.platform.safe_zone().margins.is_some()
    }

    /// The interface as a transparent layer of `resolution`, to lay over
    /// every frame of the render ([`Overlay::lay`]).
    pub fn draw(self, resolution: Resolution) -> Frame {
        let mut frame = Frame::black(resolution);
        frame.fill_transparent();
        let mut pen = Pen::new(&mut frame);
        chrome::shade(&mut pen);
        if self.safe_zones
            && let Some(margins) = self.platform.safe_zone().margins
        {
            chrome::tint(&mut pen, &margins);
        }
        match self.platform {
            Platform::InstagramReels | Platform::InstagramReelsAd => {
                instagram::reels(&mut pen, self.platform.is_ad());
            }
            Platform::InstagramStoriesAd => instagram::story_ad(&mut pen),
            Platform::YoutubeShorts => shorts::draw(&mut pen),
            Platform::Tiktok | Platform::TiktokAd => tiktok::draw(&mut pen, self.platform.is_ad()),
            Platform::Youtube => {}
        }
        frame
    }

    /// Lays a layer [`Overlay::draw`] made over a finished `frame`.
    pub fn lay(layer: &Frame, frame: &mut Frame) {
        scorsese_compositor::interface::over(frame, layer);
    }

    /// What was drawn over the picture, said for whoever reads the report:
    /// that the controls are approximate, where the tint comes from, or why
    /// there is none.
    pub fn said(self) -> String {
        let zone = self.platform.safe_zone();
        let mut said = format!(
            "the {} interface is drawn over every frame — a preview, not a file to deliver; \
             the controls are an approximation scorsese draws, not the app's own",
            self.platform
        );
        match (self.safe_zones, zone.margins) {
            (false, _) => {}
            (true, Some(_)) => said.push_str(&format!(
                "; the tinted safe zones are the vendor's figures, read {} from {}",
                zone.source.read, zone.source.page
            )),
            (true, None) => said.push_str(&format!(
                "; no safe zone is tinted, because the vendor publishes no figures for it: {}",
                zone.source.says
            )),
        }
        said
    }
}

impl fmt::Display for Overlay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.platform.id())
    }
}

/// Every placement an overlay can be drawn for, comma-separated.
fn upright() -> String {
    let ids: Vec<&str> = Platform::ALL
        .into_iter()
        .filter(|platform| *platform != Platform::Youtube)
        .map(Platform::id)
        .collect();
    ids.join(", ")
}
