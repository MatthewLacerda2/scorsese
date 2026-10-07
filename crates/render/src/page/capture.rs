//! One capture: a page driven frame by frame, its pictures encoded as they come.
//!
//! The method is #606's, as #772 re-measured it on CI: a target created with
//! begin-frame control, the clock shim injected before the page's own scripts,
//! a transparent default background (or the alpha is lost), and then — once the
//! page has loaded, laid out and its fonts are ready, and two priming frames
//! have been drawn — for each frame: advance the clock, draw one frame with
//! `HeadlessExperimental.beginFrame`, and take its screenshot in the same round
//! trip. Some of those frames are then measured for layout mistakes
//! (`layout`'s). Each PNG is piped straight into an ffmpeg encoding a lossless,
//! alpha-carrying file, so no frame is ever written to disk on its own.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Stdio};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::PageError;
use super::browser::{Chrome, ChromeError};
use super::cdp::{Cdp, CdpError, Command};
use super::layout::{self, Layout};
use super::origin::url_of;
use super::request::Request;
use super::visitor::{OFFLINE_BINDING, Visitor};
use crate::error::Stage;
use crate::tools::Tools;

/// How long any one answer may take. A frame at 4K under SwiftShader is well
/// under a second; a page stuck in a loop never answers at all.
const PATIENCE: Duration = Duration::from_secs(60);

/// How many frames are drawn before the first that counts.
const PRIMING: u32 = 2;

/// How many times a frame that came back with no picture is drawn again before
/// it is taken to have none.
const REDRAWS: u32 = 3;

/// What a finished capture heard on the way.
pub(crate) struct Heard {
    pub(crate) loaded: std::collections::BTreeMap<String, Option<String>>,
    pub(crate) warnings: Vec<String>,
}

/// Captures `request` into the file `out`, a lossless video with alpha.
pub(crate) fn run(
    chrome: &Chrome,
    tools: &Tools,
    project_root: &Path,
    follow: &[PathBuf],
    fonts: &Path,
    request: &Request,
    out: &Path,
) -> Result<Heard, PageError> {
    let launched = chrome.launch(fonts, request.viewport().scale)?;
    let process = launched.process;
    let mut cdp = Cdp::new(
        launched.to,
        launched.from,
        Visitor::new(project_root, follow),
        PATIENCE,
    );
    let heard = started(&mut cdp, chrome).and_then(|()| drive(&mut cdp, tools, request, out));
    // Asked to close rather than killed, so its helpers go with it; the
    // process is reaped either way when `process` drops.
    let _ = cdp.browser("Browser.close", json!({}));
    process.stop(Duration::from_secs(5));
    heard
}

/// Asks the browser its version before anything else, so a browser that died
/// on launch is told apart from one that died mid-capture: with the sandbox on,
/// that is nearly always a sandbox that could not start, and the error says how
/// to opt out ([`super::ChromeError::Sandbox`]) rather than only that the
/// connection closed.
fn started(cdp: &mut Cdp<ChildStdin, Visitor<'_>>, chrome: &Chrome) -> Result<(), PageError> {
    match cdp.browser("Browser.getVersion", json!({})) {
        Ok(_) => Ok(()),
        Err(CdpError::Closed) if chrome.is_sandboxed() => Err(ChromeError::Sandbox.into()),
        Err(error) => Err(error.into()),
    }
}

fn drive(
    cdp: &mut Cdp<ChildStdin, Visitor<'_>>,
    tools: &Tools,
    request: &Request,
    out: &Path,
) -> Result<Heard, PageError> {
    let viewport = request.viewport();
    let target = cdp.browser(
        "Target.createTarget",
        json!({ "url": "about:blank", "width": viewport.width, "height": viewport.height,
                "enableBeginFrameControl": true }),
    )?;
    let session = cdp.browser(
        "Target.attachToTarget",
        json!({ "targetId": target["targetId"], "flatten": true }),
    )?["sessionId"]
        .as_str()
        .map(str::to_owned);
    let mut page = |method: &'static str, params: Value| {
        cdp.call(&Command {
            method,
            params,
            session: session.clone(),
        })
    };
    page("Page.enable", json!({}))?;
    page("Runtime.enable", json!({}))?;
    // Not for requests, which `Fetch` has: for the WebSocket a page opens,
    // which only the network domain reports, and the binding the preamble
    // calls when it opens a WebRTC connection (#839).
    page("Network.enable", json!({}))?;
    page("Runtime.addBinding", json!({ "name": OFFLINE_BINDING }))?;
    page(
        "Fetch.enable",
        json!({ "patterns": [{ "urlPattern": "*" }] }),
    )?;
    page(
        "Emulation.setDeviceMetricsOverride",
        json!({ "width": viewport.width, "height": viewport.height,
                "deviceScaleFactor": viewport.scale, "mobile": false }),
    )?;
    page(
        "Emulation.setDefaultBackgroundColorOverride",
        json!({ "color": { "r": 0, "g": 0, "b": 0, "a": 0 } }),
    )?;
    page(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": request.preamble() }),
    )?;
    let navigated = page("Page.navigate", json!({ "url": url_of(&request.page) }))?;
    if let Some(error) = navigated["errorText"].as_str().filter(|e| !e.is_empty()) {
        return Err(PageError::Load(error.to_owned()));
    }
    cdp.until("Page.loadEventFired", |visitor| visitor.loaded_event)?;
    let mut page = |method: &'static str, params: Value| {
        cdp.call(&Command {
            method,
            params,
            session: session.clone(),
        })
    };
    let evaluate = |expression: &str| json!({ "expression": expression, "awaitPromise": true });
    page(
        "Runtime.evaluate",
        evaluate(
            "document.body && document.body.offsetHeight; document.fonts.ready.then(() => true)",
        ),
    )?;
    let players = page(
        "Runtime.evaluate",
        evaluate("document.querySelectorAll('video, audio').length"),
    )?;
    let mut warnings = Vec::new();
    if players["result"]["value"].as_u64().unwrap_or(0) > 0 {
        warnings.push(
            "the page has a <video> or <audio> element, which plays on its own clock \
             rather than the clip's; put footage and sound on the timeline instead"
                .to_owned(),
        );
    }
    // Early frames can come back with nothing drawn. The priming frames keep
    // their pictures, so a first frame that does too still has one to reuse.
    let mut last: Option<Vec<u8>> = None;
    for _ in 0..PRIMING {
        last = draw(&mut page)?.or(last);
    }

    let mut encoder = Encoder::start(tools, request, out)?;
    let mut layout = Layout::default();
    let measure = json!({ "expression": layout::expression(), "returnByValue": true });
    for k in 0..request.frames() {
        let advance = format!("__scorsese.advanceTo({})", request.millis_at(k));
        page("Runtime.evaluate", json!({ "expression": advance }))?;
        // A frame with no damage has no screenshot: nothing moved, so the
        // picture is the one before it.
        last = draw(&mut page)?.or(last);
        let png = last.as_ref().ok_or(PageError::NoPicture)?;
        encoder.write(png)?;
        if layout::sampled(request, k) {
            let answer = page("Runtime.evaluate", measure.clone())?;
            // A page that broke the measuring (a replaced `Range`, say) only
            // goes unmeasured; its own errors are the visitor's to report.
            let findings = serde_json::from_value(answer["result"]["value"].clone());
            layout.heard(request.millis_at(k) / 1000.0, findings.unwrap_or_default());
        }
    }
    encoder.finish()?;
    let visitor = cdp.listener();
    warnings.splice(0..0, visitor.warnings.iter().cloned());
    warnings.extend(layout.notes());
    Ok(Heard {
        loaded: visitor.loaded.clone(),
        warnings,
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
fn draw(
    page: &mut impl FnMut(&'static str, Value) -> Result<Value, CdpError>,
) -> Result<Option<Vec<u8>>, PageError> {
    for _ in 0..=REDRAWS {
        let drawn = page(
            "HeadlessExperimental.beginFrame",
            json!({ "screenshot": { "format": "png", "optimizeForSpeed": true } }),
        )?;
        if let Some(png) = picture(&drawn)? {
            return Ok(Some(png));
        }
    }
    Ok(None)
}

/// The PNG a `beginFrame` answer carries, if it drew anything.
fn picture(drawn: &Value) -> Result<Option<Vec<u8>>, PageError> {
    drawn["screenshotData"]
        .as_str()
        .map(|data| STANDARD.decode(data).map_err(|_| PageError::NoPicture))
        .transpose()
}

/// The ffmpeg turning a stream of PNGs into one lossless file with alpha.
///
/// FFV1 in Matroska, as `bgra`: lossless, carries alpha, and decoded by the
/// same path as any other video with an alpha channel. #606's PNGs ran to
/// ~1.5 MB a frame; this keeps the same pixels for a fraction of that.
struct Encoder {
    child: crate::pipe::Process,
    stdin: ChildStdin,
    subject: String,
}

impl Encoder {
    fn start(tools: &Tools, request: &Request, out: &Path) -> Result<Self, PageError> {
        let rate = format!("{}/{}", request.fps.num(), request.fps.den());
        let mut child = tools
            .ffmpeg()
            .args([
                "-nostdin",
                "-v",
                "error",
                "-y",
                "-f",
                "image2pipe",
                "-framerate",
                &rate,
            ])
            .args([
                "-c:v", "png", "-i", "-", "-c:v", "ffv1", "-pix_fmt", "bgra", "-f", "matroska",
            ])
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| crate::RenderError::Spawn {
                stage: Stage::Encode,
                source,
            })?;
        let stdin = child.stdin.take().expect("stdin was piped");
        Ok(Self {
            child: crate::pipe::Process::new(child),
            stdin,
            subject: out.display().to_string(),
        })
    }

    fn write(&mut self, png: &[u8]) -> Result<(), PageError> {
        self.stdin.write_all(png).map_err(|source| {
            PageError::Render(crate::RenderError::Pipe {
                stage: Stage::Encode,
                source,
            })
        })
    }

    fn finish(self) -> Result<(), PageError> {
        drop(self.stdin);
        Ok(self.child.finish(Stage::Encode, &self.subject)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_with_no_damage_has_no_picture() {
        assert_eq!(picture(&json!({ "hasDamage": false })).unwrap(), None);
    }

    #[test]
    fn a_drawn_frame_carries_its_png() {
        let drawn = json!({ "hasDamage": true, "screenshotData": STANDARD.encode(b"png") });
        assert_eq!(picture(&drawn).unwrap(), Some(b"png".to_vec()));
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
        let png = draw(&mut browser(
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
        assert_eq!(draw(&mut browser(vec![], &mut draws)).unwrap(), None);
        assert_eq!(draws, REDRAWS + 1);
    }

    #[test]
    fn a_picture_that_does_not_decode_is_none_at_all() {
        let drawn = json!({ "screenshotData": "not base64!" });
        assert!(matches!(picture(&drawn), Err(PageError::NoPicture)));
    }
}
