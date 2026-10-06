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
use std::path::Path;
use std::process::{ChildStdin, Stdio};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::PageError;
use super::browser::Chrome;
use super::cdp::{Cdp, Command};
use super::layout::{self, Layout};
use super::origin::url_of;
use super::request::Request;
use super::visitor::{OFFLINE_BINDING, Visitor};
use crate::error::Stage;
use crate::tools::Tools;

/// How long any one answer may take. A frame at 4K under SwiftShader is well
/// under a second; a page stuck in a loop never answers at all.
const PATIENCE: Duration = Duration::from_secs(60);

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
    fonts: &Path,
    request: &Request,
    out: &Path,
) -> Result<Heard, PageError> {
    let launched = chrome.launch(fonts, request.viewport().scale)?;
    let process = launched.process;
    let mut cdp = Cdp::new(
        launched.to,
        launched.from,
        Visitor::new(project_root),
        PATIENCE,
    );
    let heard = drive(&mut cdp, tools, request, out);
    // Asked to close rather than killed, so its helpers go with it; the
    // process is reaped either way when `process` drops.
    let _ = cdp.browser("Browser.close", json!({}));
    process.stop(Duration::from_secs(5));
    heard
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
    // Early frames can come back with nothing drawn.
    for _ in 0..2 {
        page("HeadlessExperimental.beginFrame", json!({}))?;
    }

    let mut encoder = Encoder::start(tools, request, out)?;
    let mut layout = Layout::default();
    let measure = json!({ "expression": layout::expression(), "returnByValue": true });
    let mut last: Option<Vec<u8>> = None;
    for k in 0..request.frames() {
        let advance = format!("__scorsese.advanceTo({})", request.millis_at(k));
        page("Runtime.evaluate", json!({ "expression": advance }))?;
        let drawn = page(
            "HeadlessExperimental.beginFrame",
            json!({ "screenshot": { "format": "png", "optimizeForSpeed": true } }),
        )?;
        // A frame with no damage has no screenshot: nothing moved, so the
        // picture is the one before it.
        if let Some(data) = drawn["screenshotData"].as_str() {
            last = Some(STANDARD.decode(data).map_err(|_| PageError::NoPicture)?);
        }
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
