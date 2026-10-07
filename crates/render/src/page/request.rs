//! What one capture is asked for, and what the page is told about it.
//!
//! This is the **page contract** (#777 documents it for agents): before any of
//! the page's own scripts run, `window.scorsese` holds
//!
//! - `width` and `height`: the viewport, in CSS pixels;
//! - `fps`: how many frames a second are captured;
//! - `duration`: in seconds, where the page's clock is at the clip's last frame
//!   — `source_in` plus the clip's length times its speed (#789);
//! - `clips`: where every clip beside it sits, by id, as `{start, end}` in
//!   those same seconds (#810, `told`'s);
//! - `words`: when each word of a timed narration beside it is said, as
//!   `{start, end}` in those seconds, named `<clip id>/<word>` (#811).
//!
//! **The shorter side of the viewport is always 1080 CSS pixels**, whatever the
//! render's raster; the browser's device scale makes up the difference. So a
//! page written for 1920 × 1080 lays out identically in a quarter-size preview
//! and a 4K delivery, only sharper or softer — the way a title's size is a
//! fraction of the frame rather than a count of pixels.
//!
//! **A capture is the raster's size only down to a 540 short side** (#881). The
//! picture a frame comes back as is the window's size times the scale the
//! browser was launched with (`--force-device-scale-factor`), and Chromium
//! will not launch below a half: asked for less, it draws at a half. So a
//! raster whose short side is under 540 is captured with a 540 short side —
//! 960 × 540 for a 160 × 90 golden (36 times the pixels) and for a 480 × 270
//! quarter-quality preview of a 1080p film (four times) — and the compositor
//! resamples it down like any other source of the wrong size. Nothing else
//! gets the picture smaller: the emulated device scale changes what the page
//! sees as `devicePixelRatio` but not the picture's size, and neither does the
//! emulation's own `scale`, both measured on Chromium 154. The floor is
//! Chromium's, not ours; `tests/pipeline/capture_size.rs` holds it.

use std::collections::BTreeMap;

use scorsese_compositor::Resolution;
use scorsese_core::Fps;
use serde_json::json;

use super::told::{self, Span};

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
    /// Where each clip on the timeline beside the page's own sits, by id, in
    /// seconds of the page's clock. Not part of where its capture is kept:
    /// only the clips a page reads are, once it has read them (`told`'s).
    pub clips: BTreeMap<String, Span>,
    /// When each word of every timed narration beside the page is said, by
    /// `<clip id>/<word>` ([`scorsese_core::words`] has how a word is named),
    /// in the same seconds — kept, and read, exactly as `clips` are.
    pub words: BTreeMap<String, Span>,
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

    /// The step every piece of a capture starts on: the fewest frames whose
    /// time is a whole number of milliseconds.
    ///
    /// A capture's frames are kept in Matroska, whose timestamps are whole
    /// milliseconds, so frame `k` is stamped `round(k / fps)` ms. A piece that
    /// starts on a whole millisecond stamps every frame after it exactly as one
    /// file from zero would — which is what lets pieces be joined, and one read
    /// from the middle of the page, without moving a frame (#809). At 30000/1001
    /// a piece starting on frame 61 instead of 60 joins a millisecond off.
    pub(crate) fn step(&self) -> u64 {
        let (num, den) = (u64::from(self.fps.num()), u64::from(self.fps.den()));
        num / gcd(num, 1000 * den)
    }

    /// Where a piece holding frame `k` starts: `k`, back to the nearest
    /// [`Request::step`].
    pub(crate) fn piece_start(&self, k: u64) -> u64 {
        k - k % self.step()
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

    /// The script run before the page's own: the contract and the clips, the
    /// shipped fonts, then the clock.
    pub(crate) fn preamble(&self) -> String {
        let viewport = self.viewport();
        let contract = json!({
            "width": viewport.width,
            "height": viewport.height,
            "fps": self.fps.as_f64(),
            "duration": self.duration,
        });
        let clips = json!(self.clips);
        let words = json!(self.words);
        format!(
            "{}({contract}, {clips}, {words});\n{}{}{}",
            told::SCRIPT.trim_end(),
            super::fonts::declarations(super::SHIPPED_ORIGIN),
            include_str!("clock.js"),
            include_str!("offline.js")
        )
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
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
            clips: BTreeMap::from([(
                "vo".to_owned(),
                Span {
                    start: -1.0,
                    end: 0.5,
                },
            )]),
            words: BTreeMap::new(),
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
    fn a_piece_starts_on_a_whole_millisecond() {
        let at = |fps: Fps| Request {
            fps,
            ..request(64, 64)
        };
        assert_eq!(at(Fps::THIRTY).step(), 3);
        assert_eq!(at(Fps::PAL).step(), 1);
        assert_eq!(at(Fps::new(30000, 1001).unwrap()).step(), 30);
        assert_eq!(at(Fps::new(24000, 1001).unwrap()).step(), 24);
        assert_eq!(at(Fps::THIRTY).piece_start(61), 60);
        assert_eq!(at(Fps::THIRTY).piece_start(2), 0);
        for fps in [Fps::THIRTY, Fps::PAL, Fps::new(30000, 1001).unwrap()] {
            let request = at(fps);
            let ms = request.millis_at(request.step() * 7);
            assert!((ms - ms.round()).abs() < 1e-6, "{fps:?}: {ms}");
        }
    }

    #[test]
    fn the_page_is_told_its_viewport_rate_and_length_before_the_clock_starts() {
        let preamble = request(1280, 720).preamble();
        assert!(preamble.starts_with(told::SCRIPT.trim_end()));
        let called = &preamble[told::SCRIPT.trim_end().len()..];
        let contract = called.lines().next().unwrap();
        assert!(contract.contains(r#""width":1920"#), "{contract}");
        assert!(contract.contains(r#""height":1080"#), "{contract}");
        assert!(contract.contains(r#""fps":30.0"#), "{contract}");
        assert!(contract.contains(r#""duration":2.0"#), "{contract}");
        assert!(contract.contains(r#"{"vo":{"#), "{contract}");
        assert!(contract.contains(r#""start":-1.0"#), "{contract}");
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
