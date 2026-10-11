//! How a frame is drawn once the clock has moved: one way per platform.
//!
//! The clock is ours on every platform (`clock.js`): nothing a page can read
//! moves until the capture advances it, so a frame is a function of its time.
//! What differs is how the browser is then made to draw. Where Chromium has
//! begin-frame control, a frame is drawn on request and its screenshot comes
//! back in the same round trip, so the browser runs exactly one frame of its
//! own per frame of ours. On macOS it has none (#844: "BeginFrameControl is
//! not supported on MacOS yet"), and draws on its own schedule. There the page
//! is settled over two frames of the browser's — style, layout, the events an
//! animation sends — with every animation held at the clock's time in each
//! (`clock.js`'s `settle`), and then photographed. The browser is launched
//! with the rest of `--deterministic-mode` (`browser`'s), so nothing animates
//! on a thread of its own in between, and how many frames it runs beyond the
//! two changes nothing.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::PageError;
use super::cdp::CdpError;
use super::request::{SCALE_FLOOR, Viewport};

/// How many times a frame that came back with no picture is drawn again before
/// it is taken to have none.
const REDRAWS: u32 = 3;

/// How this platform's browser is made to draw a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Drawing {
    /// `HeadlessExperimental.beginFrame`: one frame drawn on request, its
    /// screenshot in the answer.
    BeginFrame,
    /// The page settled over the browser's own frames, then
    /// `Page.captureScreenshot` (#844).
    Screenshot,
}

impl Drawing {
    /// The way this build draws: macOS has no begin-frame control.
    pub(crate) const HERE: Self = if cfg!(target_os = "macos") {
        Self::Screenshot
    } else {
        Self::BeginFrame
    };

    /// Whether the browser draws only when asked — its target created with
    /// begin-frame control, and launched with `--deterministic-mode`.
    pub(crate) fn on_request(self) -> bool {
        self == Self::BeginFrame
    }

    /// Runs one frame without drawing it: style, layout, the events an
    /// animation sends when it ends — everything but the raster and the
    /// screenshot (`capture`'s module doc).
    pub(crate) fn run(
        self,
        page: &mut impl FnMut(&'static str, Value) -> Result<Value, CdpError>,
    ) -> Result<(), PageError> {
        match self {
            Self::BeginFrame => page(
                "HeadlessExperimental.beginFrame",
                json!({ "noDisplayUpdates": true }),
            )?,
            Self::Screenshot => page(
                "Runtime.evaluate",
                json!({ "expression": "__scorsese.settle()", "awaitPromise": true }),
            )?,
        };
        Ok(())
    }

    /// Draws one frame of a page laid out in `viewport` and returns its
    /// picture.
    pub(crate) fn draw(
        self,
        page: &mut impl FnMut(&'static str, Value) -> Result<Value, CdpError>,
        viewport: Viewport,
    ) -> Result<Option<Vec<u8>>, PageError> {
        match self {
            Self::BeginFrame => begin_frame(page),
            Self::Screenshot => {
                self.run(page)?;
                let shot = page("Page.captureScreenshot", screenshot(viewport))?;
                picture(&shot["data"])
            }
        }
    }
}

/// What `Page.captureScreenshot` is asked for: the whole viewport, at the size
/// a drawn frame would come back at where frames are drawn on request.
///
/// Left to itself the screenshot is the viewport times the emulated device
/// scale, which has no floor — a 160 × 90 raster is photographed at 160 × 90.
/// A drawn frame is the launch scale's, which Chromium holds at
/// [`SCALE_FLOOR`] (`request`'s doc), so the same raster comes back at
/// 960 × 540. The clip's own scale multiplies the emulated one up to that
/// floor, so a page is rasterised at the same size on every platform and
/// resampled down the same way after: drawn at 160 × 90 instead, the six page
/// goldens' edges came out visibly softer than Linux's (SSIM 0.88 against
/// the 0.95 the gate needs on one of `page_css`'s frames).
fn screenshot(viewport: Viewport) -> Value {
    let Viewport {
        width,
        height,
        scale,
    } = viewport;
    json!({
        "format": "png",
        "optimizeForSpeed": true,
        "clip": { "x": 0, "y": 0, "width": width, "height": height,
                  "scale": scale.max(SCALE_FLOOR) / scale },
    })
}

/// Draws one frame and returns its picture, drawing it again at the same
/// instant while it comes back with none.
///
/// Every frame this browser draws normally has damage — every frame of every
/// page fixture did, the static ones too — so one without is nearly always a
/// draw that missed its deadline under load (#851, two priming frames in 272 with
/// a dozen captures at once), and its change lands on the next
/// draw. Reusing the picture before it instead would show the page a frame
/// late, or, on the first frame, show its state before the clock first moved.
/// A frame still without one after [`REDRAWS`] really did not change.
fn begin_frame(
    page: &mut impl FnMut(&'static str, Value) -> Result<Value, CdpError>,
) -> Result<Option<Vec<u8>>, PageError> {
    for _ in 0..=REDRAWS {
        let drawn = page(
            "HeadlessExperimental.beginFrame",
            json!({ "screenshot": { "format": "png", "optimizeForSpeed": true } }),
        )?;
        if let Some(png) = picture(&drawn["screenshotData"])? {
            return Ok(Some(png));
        }
    }
    Ok(None)
}

/// The PNG in a base64 field of an answer, if it carries one.
fn picture(data: &Value) -> Result<Option<Vec<u8>>, PageError> {
    data.as_str()
        .map(|data| STANDARD.decode(data).map_err(|_| PageError::NoPicture))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_with_no_damage_has_no_picture() {
        assert_eq!(
            picture(&json!({ "hasDamage": false })["screenshotData"]).unwrap(),
            None
        );
    }

    #[test]
    fn a_drawn_frame_carries_its_png() {
        let drawn = json!({ "hasDamage": true, "screenshotData": STANDARD.encode(b"png") });
        assert_eq!(
            picture(&drawn["screenshotData"]).unwrap(),
            Some(b"png".to_vec())
        );
    }

    /// A browser answering each draw with the next of `answers`, then with
    /// no picture for ever, and counting the draws.
    fn browser(
        answers: Vec<Value>,
        draws: &mut u32,
    ) -> impl FnMut(&'static str, Value) -> Result<Value, CdpError> + '_ {
        let mut answers = answers.into_iter();
        move |method, _| {
            assert_eq!(method, "HeadlessExperimental.beginFrame");
            *draws += 1;
            Ok(answers
                .next()
                .unwrap_or_else(|| json!({ "hasDamage": false })))
        }
    }

    #[test]
    fn a_frame_that_missed_is_drawn_again_until_it_has_a_picture() {
        let missed = json!({ "hasDamage": false });
        let drawn = json!({ "hasDamage": true, "screenshotData": STANDARD.encode(b"png") });
        let mut draws = 0;
        let png = begin_frame(&mut browser(
            vec![missed.clone(), missed, drawn],
            &mut draws,
        ))
        .unwrap();
        assert_eq!(png, Some(b"png".to_vec()));
        assert_eq!(draws, 3);
    }

    #[test]
    fn a_frame_that_never_changes_is_given_up_on() {
        let mut draws = 0;
        assert_eq!(begin_frame(&mut browser(vec![], &mut draws)).unwrap(), None);
        assert_eq!(draws, REDRAWS + 1);
    }

    #[test]
    fn a_picture_that_does_not_decode_is_none_at_all() {
        let drawn = json!({ "screenshotData": "not base64!" });
        assert!(matches!(
            picture(&drawn["screenshotData"]),
            Err(PageError::NoPicture)
        ));
    }
}
