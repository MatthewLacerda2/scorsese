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

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::{Path, PathBuf};

use scorsese_core::words::Words;
use scorsese_core::{AssetKind, Clip, ClipId, Fps, Project};

use crate::page::{self, Chrome, Request, Span, Wanted};
use crate::plan::{Plan, Shot};
use crate::report::Note;
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// Which browser a render's pages are captured with, and whether they are
/// captured at all or only read from `cache/`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Browser<'c> {
    /// The one given, or `None` to find one.
    pub(super) chrome: Option<&'c Chrome>,
    /// False for a renderer that only reads captures already made.
    pub(super) capturing: bool,
}

/// How many frames a clip's stretch of its page is widened by, either side:
/// a decoder seeking to an instant reads the frame either side of it.
const SPARE: u64 = 2;

/// Each page clip's captured frames, by clip.
#[derive(Debug, Default)]
pub(super) struct Pages {
    captured: HashMap<ClipId, (PathBuf, f64)>,
}

impl Pages {
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
    /// browser is looked for at all.
    pub(super) fn capture(
        tools: &Tools,
        Browser { chrome, capturing }: Browser<'_>,
        settings: &RenderSettings,
        plan: &Plan<'_>,
        project_root: &Path,
    ) -> (Self, Vec<Note>) {
        let wanted = asked(settings, plan, project_root);
        let mut pages = Self::default();
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
        for (shot, Wanted { request, frames }) in wanted {
            let captured = chrome.clone().and_then(|chrome| {
                if capturing {
                    return page::capture_frames(
                        chrome,
                        tools,
                        project_root,
                        &[],
                        &request,
                        frames,
                        page::browsers(),
                    )
                    .map_err(|e| e.to_string());
                }
                page::cached_frames(project_root, &request, chrome.version(), frames)
                    .ok_or_else(|| "not captured yet".to_owned())
            });
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

/// What a page clip asks to be captured as: the render's raster and rate, the
/// page's clock run from zero to where it is at the clip's last frame —
/// `source_in` plus the clip's length at its speed (#789) — and where the clips
/// beside it sit on that clock, and the words they say ([`beside`]). The
/// capture is then played like footage, from `source_in`, at `speed`, by the
/// ordinary decode.
pub(super) fn request_for(
    shot: &Shot<'_>,
    settings: &RenderSettings,
    plan: &Plan<'_>,
    project_root: &Path,
) -> Option<Request> {
    let clip = shot.clip;
    let timeline_fps = plan.timeline_fps();
    let duration = timeline_fps.seconds(clip.source_in)
        + timeline_fps.seconds(clip.duration) * clip.speed.get();
    let (clips, words) = beside(shot, plan.project(), timeline_fps, project_root);
    Some(Request {
        page: shot.asset.path.as_ref()?.as_str().to_owned(),
        resolution: settings.resolution,
        fps: settings.fps,
        duration,
        clips,
        words,
    })
}

/// Where every clip on the timeline a page clip sits on is, by id, in seconds
/// of the page's own clock (#810): timeline time `t` is page time `source_in +
/// (t − start) × speed`. The page clip itself is among them. Beside them, when
/// each word of every timed narration among them is said (#811), by `<clip
/// id>/<word>` on the same clock — a line with no timings names no words.
///
/// The timeline its clip sits on is the project's for a page on its tracks,
/// and a group's own for one of its members — whose clock is the group's, the
/// same wherever the group is placed, so a page in a group times itself to the
/// clips beside it there and is captured once however often the group is used.
fn beside(
    shot: &Shot<'_>,
    project: &Project,
    fps: Fps,
    project_root: &Path,
) -> (BTreeMap<String, Span>, BTreeMap<String, Span>) {
    let page = shot.clip;
    let on_timeline = project.tracks.iter().any(|track| &track.id == shot.track);
    let beside: Vec<&Clip> = if on_timeline {
        project.clips().map(|(_, clip)| clip).collect()
    } else {
        project
            .assets
            .iter()
            .filter_map(|asset| asset.group.as_ref())
            .find(|group| group.tracks.iter().any(|track| &track.id == shot.track))
            .map(|group| group.clips().map(|(_, clip)| clip).collect())
            .unwrap_or_default()
    };
    let opens = fps.seconds(page.source_in);
    let speed = page.speed.get();
    let starts = fps.seconds(page.start);
    let at = |seconds: f64| opens + (seconds - starts) * speed;
    let span = |start: f64, end: f64| Span {
        start: at(start),
        end: at(end),
    };
    let mut clips = BTreeMap::new();
    let mut words = BTreeMap::new();
    for clip in beside {
        let start = fps.seconds(clip.start);
        clips.insert(
            clip.id.to_string(),
            span(start, start + fps.seconds(clip.duration)),
        );
        let said = project
            .asset(&clip.asset)
            .and_then(|asset| Words::of(asset, project_root));
        for word in said.iter().flat_map(|said| said.placed(clip, fps)) {
            words.insert(
                format!("{}/{}", clip.id, word.name),
                span(word.start, word.end),
            );
        }
    }
    (clips, words)
}
