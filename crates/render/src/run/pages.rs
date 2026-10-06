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
//! without one never asks whether a browser exists.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use scorsese_core::{AssetKind, ClipId, Fps};

use crate::page::{self, Chrome, Request};
use crate::plan::{Plan, Shot};
use crate::report::Note;
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// Each page clip's captured frames, by clip.
#[derive(Debug, Default)]
pub(super) struct Pages {
    captured: HashMap<ClipId, PathBuf>,
}

impl Pages {
    /// The frames captured for this clip, if it is a page that was captured.
    pub(super) fn of(&self, shot: &Shot<'_>) -> Option<&Path> {
        if shot.asset.kind != AssetKind::Html {
            return None;
        }
        self.captured.get(&shot.clip.id).map(PathBuf::as_path)
    }

    /// Captures every page `plan` shows, with `chrome` or — when none was
    /// given — whichever browser [`Chrome::discover`] finds.
    pub(super) fn capture(
        tools: &Tools,
        chrome: Option<&Chrome>,
        settings: &RenderSettings,
        plan: &Plan<'_>,
        project_root: &Path,
    ) -> (Self, Vec<Note>) {
        let mut wanted: Vec<&Shot<'_>> = Vec::new();
        for segment in plan.segments() {
            for shot in &segment.layers {
                pages_in(shot, &mut wanted);
            }
        }
        let mut pages = Self::default();
        let mut notes = Vec::new();
        if wanted.is_empty() {
            return (pages, notes);
        }
        let found;
        let chrome = match chrome {
            Some(chrome) => Ok(chrome),
            None => {
                found = Chrome::discover();
                found.as_ref().map_err(ToString::to_string)
            }
        };
        for shot in wanted {
            if pages.captured.contains_key(&shot.clip.id) {
                continue;
            }
            let Some(request) = request_for(shot, settings, plan.timeline_fps()) else {
                continue;
            };
            let captured = chrome.clone().and_then(|chrome| {
                page::capture(chrome, tools, project_root, &request).map_err(|e| e.to_string())
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
                    pages.captured.insert(shot.clip.id.clone(), captured.file);
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

/// What a page clip asks to be captured as: the render's raster and rate, and
/// the page's clock run from zero to where it is at the clip's last frame —
/// `source_in` plus the clip's length at its speed (#789). The capture is then
/// played like footage, from `source_in`, at `speed`, by the ordinary decode.
pub(super) fn request_for(
    shot: &Shot<'_>,
    settings: &RenderSettings,
    timeline_fps: Fps,
) -> Option<Request> {
    let clip = shot.clip;
    let duration = timeline_fps.seconds(clip.source_in)
        + timeline_fps.seconds(clip.duration) * clip.speed.get();
    Some(Request {
        page: shot.asset.path.as_ref()?.as_str().to_owned(),
        resolution: settings.resolution,
        fps: settings.fps,
        duration,
    })
}
