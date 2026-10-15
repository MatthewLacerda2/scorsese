//! One capture: a page driven frame by frame, its pictures encoded as they come.
//!
//! The method is #606's, as #772 re-measured it on CI: a target created with
//! begin-frame control, the clock shim injected before the page's own scripts,
//! a transparent default background (or the alpha is lost), and then — once the
//! page has loaded, laid out and its fonts are ready, and two priming frames
//! have been drawn — for each frame: advance the clock, draw one frame with
//! `HeadlessExperimental.beginFrame`, and take its screenshot in the same round
//! trip. On macOS, which has no begin-frame control, the frame is the
//! browser's own and the screenshot is asked for after it (`drawing`'s, #844).
//! Some of those frames are then measured for layout mistakes
//! (`layout`'s). Each PNG is piped straight into an ffmpeg encoding a lossless,
//! alpha-carrying file, so no frame is ever written to disk on its own.
//!
//! **A capture may start part-way through the page** (#809). The frames before
//! it are run but not drawn: each gets the advance it would have had, then a
//! `beginFrame` with `noDisplayUpdates` (on macOS, the browser's next frames
//! of its own), which runs the browser's own frame —
//! style, layout, the events an animation sends when it ends — and skips the
//! raster and the screenshot, nearly all of a frame's cost. So the page
//! arrives at its first frame in the state it would have reached frame by
//! frame: a page that adds `speed * dt` every frame, an animation a timer
//! starts, one started from another's `animationend`. A jump of the clock
//! would get all three wrong, silently; advancing without the browser's frame
//! gets the last one wrong (`same_frames` holds both). What it cannot repeat is
//! the compositor's history — which layers the frames before were drawn in —
//! and that shows as anti-aliasing a pixel apart on a diagonal edge at most,
//! inside the golden renders' tolerance and smaller than two captures from the
//! first frame already differ by (#809's PR has the numbers).

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::ChildStdin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use super::PageError;
use super::browser::{Chrome, ChromeError};
use super::cdp::{Cdp, CdpError, Command};
use super::drawing::Drawing;
use super::encoder::Encoder;
use super::layout::{self, Layout};
use super::origin::url_of;
use super::request::Request;
use super::told::{self, Read};
use super::visitor::{OFFLINE_BINDING, Visitor};
use crate::cancel::Cancel;
use crate::tools::Tools;

/// How long any one answer may take. A frame at 4K under SwiftShader is well
/// under a second; a page stuck in a loop never answers at all.
const PATIENCE: Duration = Duration::from_secs(60);

/// How many frames are drawn before the first that counts.
const PRIMING: u32 = 2;

/// What a finished capture heard on the way.
pub(crate) struct Heard {
    pub(crate) loaded: std::collections::BTreeMap<String, Option<String>>,
    pub(crate) warnings: Vec<String>,
    /// What the page read of `scorsese.clips` (`told`'s).
    pub(crate) read: Read,
}

/// What a capture serves the page from: its project, the folders a link out
/// of it may lead to (`super::capture_following`), and the shipped fonts'
/// fontconfig file.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Served<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) follow: &'a [PathBuf],
    pub(crate) fonts: &'a Path,
}

/// Captures `frames` of `request` into the file `out`, a lossless video with
/// alpha whose first frame is the first of `frames`.
///
/// Its layout is measured from frame `measured` on — run or drawn — at the
/// samples a capture from the first frame would take (`layout::sampled`), and
/// at the last frame drawn too: a still's few frames would hold no mistake
/// otherwise, since one is only said once two samples in a row agree on it.
/// So the first piece of a capture measures from zero and a later one from its
/// own first frame, and between them they say what one capture would have.
///
/// Each frame drawn into `out` adds one to `drawn`, which other pieces of the
/// same capture may be adding to at once: how a render says how far its pages
/// have got (#1055). `cancel` is looked at before each frame, run or drawn, and
/// stops it with [`PageError::Cancelled`] (#1088).
pub(crate) fn run(
    chrome: &Chrome,
    tools: &Tools,
    served: Served<'_>,
    request: &Request,
    (frames, measured): (Range<u64>, u64),
    (out, drawn, cancel): (&Path, &AtomicU64, &Cancel),
) -> Result<Heard, PageError> {
    let Served {
        project_root,
        follow,
        fonts,
    } = served;
    let launched = chrome.launch(fonts, request.viewport().scale)?;
    let process = launched.process;
    let mut cdp = Cdp::new(
        launched.to,
        launched.from,
        Visitor::new(project_root, follow),
        PATIENCE,
    );
    let heard = started(&mut cdp, chrome).and_then(|()| {
        let to = (out, drawn, cancel);
        drive(&mut cdp, tools, request, (frames, measured), to)
    });
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
    (frames, measured): (Range<u64>, u64),
    (out, drawn, cancel): (&Path, &AtomicU64, &Cancel),
) -> Result<Heard, PageError> {
    let stopped = || {
        if cancel.is_cancelled() {
            Err(PageError::Cancelled)
        } else {
            Ok(())
        }
    };
    let viewport = request.viewport();
    let target = cdp.browser(
        "Target.createTarget",
        json!({ "url": "about:blank", "width": viewport.width, "height": viewport.height,
                "enableBeginFrameControl": Drawing::HERE.on_request() }),
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
    // Chrome's own generic families are Arial, Times New Roman and Monospace,
    // which it answers with their metric twins (Liberation) or fontconfig's
    // first face; naming the generics leaves them to the fonts file, which
    // binds each to the face it means (`cache`'s `GENERICS`, #1030).
    page(
        "Page.setFontFamilies",
        json!({ "fontFamilies":
            { "sansSerif": "sans-serif", "serif": "serif", "fixed": "monospace" } }),
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
        last = Drawing::HERE.draw(&mut page, viewport)?.or(last);
    }

    let mut layout = Layout::default();
    let measure = json!({ "expression": layout::expression(), "returnByValue": true });
    let mut measured_at = |page: &mut dyn FnMut(&'static str, Value) -> Result<Value, CdpError>,
                           k: u64|
     -> Result<(), PageError> {
        let answer = page("Runtime.evaluate", measure.clone())?;
        // A page that broke the measuring (a replaced `Range`, say) only
        // goes unmeasured; its own errors are the visitor's to report.
        let findings = serde_json::from_value(answer["result"]["value"].clone());
        layout.heard(request.millis_at(k) / 1000.0, findings.unwrap_or_default());
        Ok(())
    };
    // Run, not drawn: see the module doc.
    for k in 0..frames.start {
        stopped()?;
        let advance = format!("__scorsese.advanceTo({})", request.millis_at(k));
        page("Runtime.evaluate", json!({ "expression": advance }))?;
        Drawing::HERE.run(&mut page)?;
        if k >= measured && layout::sampled(request, k) {
            measured_at(&mut page, k)?;
        }
    }

    let mut encoder = Encoder::start(tools, request, out)?;
    let (first, end) = (frames.start, frames.end);
    for k in frames {
        stopped()?;
        let advance = format!("__scorsese.advanceTo({})", request.millis_at(k));
        page("Runtime.evaluate", json!({ "expression": advance }))?;
        // A frame with no damage has no screenshot: nothing moved, so the
        // picture is the one before it.
        last = Drawing::HERE.draw(&mut page, viewport)?.or(last);
        let png = last.as_ref().ok_or(PageError::NoPicture)?;
        encoder.write(png)?;
        drawn.fetch_add(1, Ordering::SeqCst);
        // A piece measured only from its own first frame measures that one
        // too, so a mistake already on screen is held from there.
        let starts = k == first && k == measured;
        if starts || k + 1 == end || layout::sampled(request, k) {
            measured_at(&mut page, k)?;
        }
    }
    encoder.finish()?;
    // Asked once, after the last frame: by then the page has read everything
    // it drew with. A page that broke the asking is taken to have read
    // everything — the safe side, since it is captured again more often.
    let asked = page(
        "Runtime.evaluate",
        json!({ "expression": told::ASK, "returnByValue": true }),
    )?;
    let read = serde_json::from_value(asked["result"]["value"].clone()).unwrap_or(Read {
        listed: true,
        listed_words: true,
        ..Read::default()
    });
    let visitor = cdp.listener();
    warnings.splice(0..0, visitor.warnings.iter().cloned());
    warnings.extend(layout.notes());
    Ok(Heard {
        loaded: visitor.loaded.clone(),
        warnings,
        read,
    })
}
