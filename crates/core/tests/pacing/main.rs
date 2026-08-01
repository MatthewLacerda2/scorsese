//! Scaling clip positions about an instant: the pacing operation.
//!
//! Asserted on the **document that comes out**, because that is the whole of
//! what this does — it moves numbers in `project.json` and refuses to move them
//! into a shape that would not load. No window and no render is involved in
//! any of it.

mod durations;
mod refusals;

use std::collections::BTreeSet;

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, GenerationState, Project, ProjectPath,
    Track, TrackId, TrackKind,
};

/// A cut with air in it, and one clip of each kind that matters here.
///
/// ```text
/// v1  [--head--][--tail--]              [-logo-]
/// v2                            [plate]
/// a1  [--------------- score ------------------]
/// a2        [-line-]
///     0     60     120    180    240    300  360
/// ```
///
/// `head` and `tail` touch exactly at 120, which is the property worth
/// protecting. `plate` and `score` have files behind them; everything else is
/// a title, a still, or a brief nobody has generated.
pub(crate) fn project() -> Project {
    let mut project = Project::new("pacing", Fps::THIRTY);
    project.assets = vec![
        Asset::text(AssetId::new("words"), "SCORSESE"),
        Asset::imported(
            AssetId::new("still"),
            AssetKind::Image,
            ProjectPath::new("assets/logo.png"),
        ),
        Asset::imported(
            AssetId::new("footage"),
            AssetKind::Video,
            ProjectPath::new("assets/plate.mp4"),
        ),
        Asset::sketch(AssetId::new("brief"), AssetKind::GeneratedAudio, "a line"),
        baked(),
    ];
    project.tracks = vec![
        track(
            "v1",
            TrackKind::Video,
            vec![
                clip("head", "words", 0, 120),
                clip("tail", "words", 120, 60),
                clip("logo", "still", 300, 60),
            ],
        ),
        track(
            "v2",
            TrackKind::Video,
            vec![clip("plate", "footage", 240, 60)],
        ),
        track("a1", TrackKind::Audio, vec![clip("score", "music", 0, 360)]),
        track("a2", TrackKind::Audio, vec![clip("line", "brief", 60, 60)]),
    ];
    project
}

/// A synthesised score that has already been baked — so the file exists, and
/// its length is the file's business rather than the timeline's.
fn baked() -> Asset {
    let mut asset = Asset::synth(
        AssetId::new("music"),
        AssetKind::SynthAudio,
        ProjectPath::new("recipes/theme.json"),
    );
    asset.state = Some(GenerationState::Generated);
    asset.path = Some(ProjectPath::new("generated/theme.wav"));
    asset
}

pub(crate) fn track(id: &str, kind: TrackKind, clips: Vec<Clip>) -> Track {
    Track {
        note: None,
        id: TrackId::new(id),
        kind,
        name: None,
        clips,
    }
}

pub(crate) fn clip(id: &str, asset: &str, start: u64, duration: u64) -> Clip {
    Clip::new(
        ClipId::new(id),
        AssetId::new(asset),
        Frames(start),
        Frames(duration),
    )
}

/// A selection, as the window and the MCP tool both hand one in.
pub(crate) fn selection(ids: &[&str]) -> BTreeSet<ClipId> {
    ids.iter().map(|id| ClipId::new(*id)).collect()
}

/// Where a clip sits now, as `(start, duration)`.
pub(crate) fn at(project: &Project, id: &str) -> (u64, u64) {
    project
        .clips()
        .find(|(_, clip)| clip.id.as_str() == id)
        .map(|(_, clip)| (clip.start.get(), clip.duration.get()))
        .expect("the clip is in this project")
}
