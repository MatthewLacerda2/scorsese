//! A web render says what the CLI says after its file (#1289).

use std::collections::HashMap;

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, GenerationState, Project, ProjectPath,
    Track, TrackId, TrackKind,
};
use scorsese_render::{Description, FrameRange, Note, Plan, RenderReport};

use serde_json::json;

use super::said;

/// A song of four beats: what a bake reads, here only to be read.
const SONG: &str = r#"{
  "recipe": "song", "bpm": 120, "seed": 3,
  "tracks": [{ "name": "pad", "patch": { "source": { "kind": "karplus" },
      "amp": { "a": 0.001, "d": 0.2, "s": 0.0, "r": 0.1 } } }],
  "patterns": { "a": { "beats": 4, "notes": [
      { "track": "pad", "note": "C4", "start": 0, "dur": 1 } ] } },
  "arrangement": ["a"]
}"#;

/// A project whose music claims a bake its recipe no longer makes, laid out
/// at a fresh folder with the recipe in it.
fn stale_music() -> (Project, std::path::PathBuf) {
    let root = crate::scratch::dir("said-stale");
    std::fs::create_dir_all(root.join("recipes")).expect("the test setup works");
    std::fs::write(root.join("recipes/theme.json"), SONG).expect("the test setup works");
    let mut project = Project::new("p", Fps::THIRTY);
    let id = AssetId::new("theme");
    let mut theme = Asset::synth(
        id.clone(),
        AssetKind::SynthAudio,
        ProjectPath::new("recipes/theme.json"),
    );
    theme.state = Some(GenerationState::Generated);
    theme.path = Some(ProjectPath::new("generated/theme-older.wav"));
    project.assets.push(theme);
    let mut music = Track::new(TrackId::new("a1"), TrackKind::Audio);
    music
        .clips
        .push(Clip::new(ClipId::new("m"), id, Frames(0), Frames(60)));
    project.tracks.push(music);
    (project, root)
}

/// A report of a silent render with `notes`, as the renderer hands one back.
fn report(notes: Vec<Note>) -> RenderReport {
    let mut card = serde_json::to_value(Project::new("card", Fps::THIRTY)).expect("serialises");
    card["assets"] = json!([{ "id": "bg", "kind": "color", "color": "#336699" }]);
    card["tracks"] = json!([{ "id": "v1", "kind": "video",
        "clips": [{ "id": "c1", "asset": "bg", "start": 0, "duration": 6 }] }]);
    let card = Project::from_json(&card.to_string()).expect("the card is a project");
    let plan = Plan::build(&card, Fps::THIRTY, FrameRange::ALL).expect("a plan");
    RenderReport {
        frames: 0,
        fps: Fps::THIRTY,
        resolution: None,
        seconds_of_audio: None,
        levels: None,
        delivered: None,
        trim: None,
        lift: None,
        notes,
        description: Description::of(&plan),
    }
}

#[test]
fn a_render_that_plays_a_stale_bake_says_so_in_the_clis_words() {
    let (project, root) = stale_music();
    let warned = Note::PageWarning {
        asset: "title".into(),
        warning: "it asked for the internet".into(),
    };

    let said = said(
        report(vec![warned.clone()]),
        &project,
        &root,
        &HashMap::new(),
    );

    assert_eq!(said.len(), 2, "{said:?}");
    assert_eq!(said[0], format!("note: {warned}"));
    for says in [
        "note: synth asset `theme`",
        "older than its recipe",
        "synth_bake",
    ] {
        assert!(said[1].contains(says), "{says}: {said:?}");
    }
}

#[test]
fn a_render_with_nothing_stale_has_nothing_more_to_say() {
    let (mut project, root) = stale_music();
    project.tracks.clear();
    assert!(said(report(Vec::new()), &project, &root, &HashMap::new()).is_empty());
}
