//! What one capture is asked for, and what the page is told about it.
//!
//! This is the **page contract** (#777 documents it for agents): before any of
//! the page's own scripts run, `window.scorsese` holds
//!
//! - `width` and `height`: the viewport, in CSS pixels;
//! - `fps`: how many frames a second are captured;
//! - `duration`: in seconds, where the page's clock is at the clip's last frame
//!   — `source_in` plus the clip's length times its speed (#789).
//!
//! **The shorter side of the viewport is always 1080 CSS pixels**, whatever the
//! render's raster; the browser's device scale makes up the difference. So a
//! page written for 1920 × 1080 lays out identically in a quarter-size preview
//! and a 4K delivery, only sharper or softer — the way a title's size is a
//! fraction of the frame rather than a count of pixels.

use scorsese_compositor::Resolution;
use scorsese_core::Fps;
use serde_json::json;

/// The shorter side of every page's viewport, in CSS pixels.
pub(crate) const SHORT_SIDE: u32 = 1080;

/// One page, captured at one raster and rate for one length of clock.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The page's path from the project root, as its asset records it.
    pub page: String,
    /// The raster the frames are captured at — the render's own.
    pub resolution: Resolution,
    /// The rate they are captured at — the render's own.
    pub fps: Fps,
    /// How far the page's clock runs, in seconds, from zero.
    pub duration: f64,
}

/// The viewport a page lays out in, and the scale that brings it to the raster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Viewport {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) scale: f64,
}

impl Request {
    /// How many frames the capture holds: every frame of the clock from zero
    /// up to `duration`, and one past it, so the frame a decoder rounds onto at
    /// the clip's very end is always there.
    pub fn frames(&self) -> u64 {
        let frames = (self.duration.max(0.0) * self.fps.as_f64()).ceil() as u64;
        frames + 1
    }

    /// The milliseconds of page clock at capture frame `k`.
    pub(crate) fn millis_at(&self, k: u64) -> f64 {
        self.fps.seconds_at(k as f64) * 1000.0
    }

    pub(crate) fn viewport(&self) -> Viewport {
        let (width, height) = (self.resolution.width(), self.resolution.height());
        let short = width.min(height);
        let scale = f64::from(short) / f64::from(SHORT_SIDE);
        let css = |side: u32| (f64::from(side) / scale).round() as u32;
        Viewport {
            width: if width == short {
                SHORT_SIDE
            } else {
                css(width)
            },
            height: if height == short {
                SHORT_SIDE
            } else {
                css(height)
            },
            scale,
        }
    }

    /// The script run before the page's own: the contract, the shipped fonts,
    /// then the clock.
    pub(crate) fn preamble(&self) -> String {
        let viewport = self.viewport();
        let contract = json!({
            "width": viewport.width,
            "height": viewport.height,
            "fps": self.fps.as_f64(),
            "duration": self.duration,
        });
        format!(
            "Object.defineProperty(window, 'scorsese', {{ value: Object.freeze({contract}) }});\n{}{}{}",
            super::fonts::declarations(super::SHIPPED_ORIGIN),
            include_str!("clock.js"),
            include_str!("offline.js")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(width: u32, height: u32) -> Request {
        Request {
            page: "pages/a.html".into(),
            resolution: Resolution::new(width, height).unwrap(),
            fps: Fps::THIRTY,
            duration: 2.0,
        }
    }

    #[test]
    fn the_short_side_is_always_1080_css_pixels() {
        let wide = request(1280, 720).viewport();
        assert_eq!((wide.width, wide.height), (1920, 1080));
        assert!((wide.scale - 2.0 / 3.0).abs() < 1e-12);
        let tall = request(1080, 1920).viewport();
        assert_eq!((tall.width, tall.height, tall.scale), (1080, 1920, 1.0));
        let tiny = request(64, 64).viewport();
        assert_eq!((tiny.width, tiny.height), (1080, 1080));
    }

    #[test]
    fn a_capture_holds_the_whole_clock_and_one_frame_past_it() {
        assert_eq!(request(64, 64).frames(), 61);
        let short = Request {
            duration: 0.01,
            ..request(64, 64)
        };
        assert_eq!(short.frames(), 2);
        assert!((request(64, 64).millis_at(3) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn the_page_is_told_its_viewport_rate_and_length_before_the_clock_starts() {
        let preamble = request(1280, 720).preamble();
        let contract = preamble.lines().next().unwrap();
        assert!(contract.contains(r#""width":1920"#), "{contract}");
        assert!(contract.contains(r#""height":1080"#), "{contract}");
        assert!(contract.contains(r#""fps":30.0"#), "{contract}");
        assert!(contract.contains(r#""duration":2.0"#), "{contract}");
        assert!(preamble.contains("__scorsese"));
    }

    #[test]
    fn one_advance_fires_timers_then_frames_then_seeks_animations() {
        // The order is the shim's whole contract with a page (see clock.js),
        // and nothing outside a browser can run it — so it is held here by
        // where each step sits in the one function that does them.
        let clock = include_str!("clock.js");
        let body = &clock[clock.find("const advanceTo").unwrap()..];
        let timers = body.find("timers").unwrap();
        let frames = body.find("const due = frames").unwrap();
        let seek = body.find("getAnimations").unwrap();
        assert!(timers < frames && frames < seek);
        assert!(
            clock.contains("const START = 100;"),
            "anime.js needs a clock above zero"
        );
    }
}
