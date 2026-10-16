//! What a page clip asks to be captured as ([`request_for`]): its page, the
//! render's raster and rate, how far its clock runs, and where the clips and
//! words beside it fall on that clock.

use std::collections::BTreeMap;
use std::path::Path;

use scorsese_core::words::Words;
use scorsese_core::{Clip, Fps, Project};

use crate::page::{Request, Span};
use crate::plan::{Plan, Shot};
use crate::settings::RenderSettings;

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
    let beside: Vec<&Clip> = project
        .stacks()
        .find(|(_, tracks)| tracks.iter().any(|track| &track.id == shot.track))
        .map(|(_, tracks)| tracks.iter().flat_map(|track| &track.clips).collect())
        .unwrap_or_default();
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
