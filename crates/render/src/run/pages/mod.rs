//! The web pages a render shows, captured before anything is drawn.
//!
//! Capturing is its own step ([`crate::page::capture`]), and this is where a
//! render takes it: every `html` clip on screen in the plan is captured — or
//! found already captured in `cache/` — at the render's raster and rate, and the
//! file it lands in is decoded like any video with alpha from then on. A clip
//! whose page could not be captured shows the page's slug card, with the reason
//! on the report: a stand-in, never a failed render (#774).
//!
//! The browser is looked for only when the plan has a page in it, so a project
//! without one never asks whether a browser exists — and a program allowed to
//! download one ([`crate::page::supply`]) downloads it only then.
//!
//! **Only the frames a clip shows are captured** (#809): a still a handful
//! around its instant, a render each clip's stretch from where it enters the
//! page to where it leaves, with a frame or two to spare either side for the
//! decoder's rounding. A clip inside a group or a matte asks for its whole
//! page, since where it is in the page is the group's to decide.
//!
//! A renderer told not to capture ([`crate::Renderer::without_capturing`]) draws
//! a page only from a capture already in `cache/`, and its card otherwise: the
//! window's preview, which captures in the background instead of making a
//! scrub wait minutes for a page (#776).
//!
//! **Distinct pages are captured at once** (#1095), sharing one budget of
//! browsers ([`together`]): a dozen short pages draw side by side instead of
//! one after another, and produce the same frames either way.
//!
//! **Capturing is a phase of the render's own** ([`crate::Phase::Capturing`], #1055),
//! counted in page frames drawn of those the cache does not already hold: on
//! a page-heavy edit it is most of the render, and a render that said it was
//! still preparing all that while read as one that had hung. An edit whose
//! pages are all in `cache/` never enters it.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

use scorsese_core::{AssetKind, ClipId, Fps};

use crate::cancel::Cancel;
use crate::page::{self, Chrome, Request, Wanted};
use crate::plan::{Plan, Shot};
use crate::progress::Progress;
use crate::report::Note;
use crate::settings::RenderSettings;
use crate::tools::Tools;

mod request;
mod together;

use request::request_for;

/// Which browser a render's pages are captured with, and whether they are
/// captured at all or only read from `cache/`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Browser<'c> {
    /// The one given, or `None` to find one.
    pub(super) chrome: Option<&'c Chrome>,
    /// False for a renderer that only reads captures already made.
    pub(super) capturing: bool,
    /// True when the captures a non-capturing renderer reads are being made
    /// elsewhere right now (#1202).
    pub(super) underway: bool,
}

/// How many frames a clip's stretch of its page is widened by, either side:
/// a decoder seeking to an instant reads the frame either side of it.
const SPARE: u64 = 2;

/// Each page clip's captured frames, by clip — held until the render is over,
/// when the cache is tidied ([`page::Holding`]), one holding for each thread
/// that captured them ([`together`]).
#[derive(Debug, Default)]
pub(super) struct Pages {
    captured: HashMap<ClipId, (PathBuf, f64)>,
    holdings: Vec<page::Holding>,
    /// Whether an absent capture is one on its way, for the card to say so.
    underway: bool,
}

impl Pages {
    /// The card a page without a capture stands on: capturing when its capture
    /// is under way elsewhere, not captured otherwise.
    pub(super) fn absence(&self, absent: crate::slug::Absent) -> crate::slug::Absent {
        use crate::slug::Absent;
        if self.underway && absent == Absent::Uncaptured {
            Absent::Capturing
        } else {
            absent
        }
    }

    /// The file captured for this clip, if it is a page that was captured, and
    /// how many seconds into the page that file begins — what a seek into it
    /// takes off, since the file's own clock starts at zero.
    pub(super) fn of(&self, shot: &Shot<'_>) -> Option<(&Path, f64)> {
        if shot.asset.kind != AssetKind::Html {
            return None;
        }
        self.captured
            .get(&shot.clip.id)
            .map(|(file, begins)| (file.as_path(), *begins))
    }

    /// Captures every page `plan` shows, with the browser given or — when none
    /// was — whichever one [`Chrome::discover`] finds. Unless the browser is
    /// not `capturing`: then only what is already captured is used, and no
    /// browser is looked for at all. Distinct pages are captured at once
    /// ([`together`]). Frames drawn are counted on `progress`,
    /// and a tripped `cancel` stops the capture under way before its next
    /// frame and every one after it before it starts (#1088).
    pub(super) fn capture(
        (tools, progress, cancel): (&Tools, &Progress, &Cancel),
        Browser {
            chrome,
            capturing,
            underway,
        }: Browser<'_>,
        settings: &RenderSettings,
        plan: &Plan<'_>,
        project_root: &Path,
    ) -> (Self, Vec<Note>) {
        let wanted = asked(settings, plan, project_root);
        let mut pages = Self {
            underway,
            ..Self::default()
        };
        let mut notes = Vec::new();
        if wanted.is_empty() {
            return (pages, notes);
        }
        let found;
        let chrome = match chrome {
            Some(chrome) => Ok(chrome),
            None if !capturing => Err("not captured yet".to_owned()),
            None => {
                found = page::find();
                if let Ok(found) = &found
                    && found.fetched
                {
                    notes.push(Note::PageRendererFetched {
                        version: found.chrome.version().to_owned(),
                    });
                }
                found
                    .as_ref()
                    .map(|found| &found.chrome)
                    .map_err(ToString::to_string)
            }
        };
        let uncounted = AtomicU64::new(0);
        let drawn = match &chrome {
            Ok(chrome) if capturing => {
                let of = wanted
                    .iter()
                    .map(|(_, one)| {
                        page::to_draw(
                            project_root,
                            &one.request,
                            chrome.version(),
                            one.frames.clone(),
                        )
                    })
                    .sum();
                if of > 0 {
                    progress.capturing(of)
                } else {
                    &uncounted
                }
            }
            _ => &uncounted,
        };
        let outcomes: Vec<together::Outcome> = match &chrome {
            Ok(chrome) if capturing => {
                let asked: Vec<_> = wanted
                    .iter()
                    .map(|(_, one)| (one.request.clone(), one.frames.clone()))
                    .collect();
                let how = together::Capturing {
                    chrome,
                    tools,
                    project_root,
                    drawn,
                    cancel,
                };
                let browsers = page::Browsers::new(page::browsers());
                let (outcomes, holdings) = together::capture(&how, &asked, &browsers);
                pages.holdings = holdings;
                outcomes
            }
            _ => {
                let mut holding = page::Holding::default();
                let outcomes = wanted
                    .iter()
                    .map(|(_, Wanted { request, frames })| {
                        let chrome = chrome.clone()?;
                        holding
                            .cached_frames(project_root, request, chrome.version(), frames.clone())
                            .ok_or_else(|| "not captured yet".to_owned())
                    })
                    .collect();
                pages.holdings.push(holding);
                outcomes
            }
        };
        for ((shot, Wanted { request, .. }), captured) in wanted.into_iter().zip(outcomes) {
            match captured {
                Ok(captured) => {
                    for warning in captured.warnings {
                        let note = Note::PageWarning {
                            asset: shot.asset.id.to_string(),
                            warning,
                        };
                        if !notes.contains(&note) {
                            notes.push(note);
                        }
                    }
                    let begins = request.fps.seconds_at(captured.first as f64);
                    pages
                        .captured
                        .insert(shot.clip.id.clone(), (captured.file, begins));
                }
                Err(reason) => notes.push(Note::PageNotCaptured {
                    clip: shot.clip.id.to_string(),
                    asset: shot.asset.id.to_string(),
                    reason,
                }),
            }
        }
        (pages, notes)
    }
}

/// Which of its page's frames each page clip on the plan's own tracks shows, at
/// `fps`, over every segment it is in — [`SPARE`] wider either side. A clip
/// only inside a group or a matte has none, and is captured whole.
fn stretches(plan: &Plan<'_>, fps: Fps) -> HashMap<ClipId, Range<u64>> {
    let timeline = plan.timeline_fps();
    let mut stretches: HashMap<ClipId, Range<u64>> = HashMap::new();
    for segment in plan.segments() {
        for shot in segment
            .layers
            .iter()
            .filter(|s| s.asset.kind == AssetKind::Html)
        {
            let enters = timeline.seconds_at(shot.source_in);
            let lasts = timeline.seconds_at(segment.duration.0 as f64) * shot.clip.speed.get();
            let first = (enters * fps.as_f64()).floor() as u64;
            let last = ((enters + lasts) * fps.as_f64()).ceil() as u64;
            let shown = first.saturating_sub(SPARE)..last + SPARE;
            stretches
                .entry(shot.clip.id.clone())
                .and_modify(|seen| *seen = seen.start.min(shown.start)..seen.end.max(shown.end))
                .or_insert(shown);
        }
    }
    stretches
}

/// Every page shot `plan` shows, its groups' members and mattes included.
fn shots<'p, 'a>(plan: &'p Plan<'a>) -> Vec<&'p Shot<'a>> {
    let mut wanted = Vec::new();
    for segment in plan.segments() {
        for shot in &segment.layers {
            pages_in(shot, &mut wanted);
        }
    }
    wanted
}

/// Each page clip `plan` shows, once, with what capturing it asks for: its
/// request, and the stretch of the page it shows ([`stretches`]).
fn asked<'p, 'a>(
    settings: &RenderSettings,
    plan: &'p Plan<'a>,
    project_root: &Path,
) -> Vec<(&'p Shot<'a>, Wanted)> {
    let stretches = stretches(plan, settings.fps);
    let mut asked: Vec<(&Shot<'_>, Wanted)> = Vec::new();
    for shot in shots(plan) {
        if asked.iter().any(|(seen, _)| seen.clip.id == shot.clip.id) {
            continue;
        }
        let Some(request) = request_for(shot, settings, plan, project_root) else {
            continue;
        };
        let frames = stretches.get(&shot.clip.id).cloned().unwrap_or(0..u64::MAX);
        asked.push((shot, Wanted::new(request, frames)));
    }
    asked
}

/// What capturing only the frames `plan` shows would ask for, once each — the
/// captures a render that does not capture itself reads ([`Pages::capture`]).
pub(super) fn wanted(
    settings: &RenderSettings,
    plan: &Plan<'_>,
    project_root: &Path,
) -> Vec<Wanted> {
    let mut wanted: Vec<Wanted> = Vec::new();
    for (_, one) in asked(settings, plan, project_root) {
        if !wanted.contains(&one) {
            wanted.push(one);
        }
    }
    wanted
}

/// What capturing every page `plan` shows would ask for, once each.
pub(super) fn requests(
    settings: &RenderSettings,
    plan: &Plan<'_>,
    project_root: &Path,
) -> Vec<Request> {
    let mut requests: Vec<Request> = Vec::new();
    for shot in shots(plan) {
        if let Some(request) = request_for(shot, settings, plan, project_root)
            && !requests.contains(&request)
        {
            requests.push(request);
        }
    }
    requests
}

/// Every page among a shot, its group's members and its matte.
fn pages_in<'s, 'a>(shot: &'s Shot<'a>, into: &mut Vec<&'s Shot<'a>>) {
    if shot.asset.kind == AssetKind::Html {
        into.push(shot);
    }
    for member in &shot.members {
        pages_in(member, into);
    }
    if let Some(matte) = &shot.matte {
        pages_in(&matte.shot, into);
    }
}
