//! The serde format: what loads, what round-trips, and what is refused.

mod common;

use scorsese_core::{
    AssetKind, Fps, Frames, GenerationState, LoadError, MediaMetadata, Project, SCHEMA_VERSION,
};

#[test]
fn fixture_parses_and_validates() {
    let project = common::project();
    assert_eq!(project.name, "Narrated teaser");
    assert_eq!(project.timeline_fps, Fps::THIRTY);
    assert_eq!(project.assets.len(), 6);
    assert_eq!(project.clips().count(), 6);
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn round_trip_is_lossless() {
    let project = common::project();
    let reparsed = Project::from_json(&project.to_json().expect("serialise")).expect("reparse");
    assert_eq!(project, reparsed);
}

#[test]
fn optional_fields_are_omitted_not_nulled() {
    let json = common::project().to_json().expect("serialise");
    assert!(
        !json.contains("null"),
        "absent fields must be omitted, not written as null"
    );
}

/// What GO would realise — which is not the same list as what GO would
/// *bill* for. `score` is synthesised locally and costs nothing, and keeping
/// the two ideas apart is the whole reason `is_prompted` exists beside
/// `is_generated`.
#[test]
fn sketch_assets_are_the_ones_go_would_realise() {
    let project = common::project();
    let pending: Vec<_> = project
        .pending_generation()
        .map(|a| a.id.as_str().to_owned())
        .collect();
    assert_eq!(pending, ["shot-city", "vo-open", "score"]);

    let billed: Vec<_> = project
        .pending_generation()
        .filter(|a| a.kind.is_prompted())
        .map(|a| a.id.as_str().to_owned())
        .collect();
    assert_eq!(billed, ["shot-city", "vo-open"], "synthesis is free");

    assert!(
        project
            .assets
            .iter()
            .all(|a| !a.has_renderable_media() || !a.needs_generation())
    );
}

#[test]
fn kinds_and_states_use_snake_case_on_the_wire() {
    let project = common::project();
    let shot = project
        .asset(&common::asset_id("shot-city"))
        .expect("asset");
    assert_eq!(shot.kind, AssetKind::GeneratedVideo);
    assert_eq!(shot.state, Some(GenerationState::Sketch));

    let json = project.to_json().expect("serialise");
    assert!(json.contains("\"generated_video\""));
    assert!(json.contains("\"sketch\""));
    assert!(json.contains("\"ease_in\""));
}

#[test]
fn source_in_defaults_to_zero() {
    let project = common::project();
    let (_, clip) = project.clips().next().expect("a clip");
    assert_eq!(clip.source_in, Frames::ZERO);
}

#[test]
fn a_probed_source_framerate_round_trips_exactly() {
    let mut project = common::project();
    common::asset_mut(&mut project, "logo").media = Some(MediaMetadata {
        frame_rate: Some(Fps::NTSC),
        ..MediaMetadata::default()
    });
    let json = project.to_json().expect("serialise");
    assert!(json.contains("30000"), "a rational rate stays rational");
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
}

#[test]
fn unknown_fields_are_refused() {
    let error = Project::from_json(&common::document(r#""trackz": []"#))
        .expect_err("unknown field must fail");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

#[test]
fn a_future_schema_version_is_refused() {
    let json = format!(
        r#"{{ "schema_version": {}, "name": "from the future" }}"#,
        SCHEMA_VERSION + 1
    );
    let error = Project::from_json(&json).expect_err("future version must fail");
    match error {
        LoadError::SchemaVersion { found, supported } => {
            assert_eq!((found, supported), (SCHEMA_VERSION + 1, SCHEMA_VERSION));
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn save_then_load_returns_the_same_project() {
    let dir = common::temp_project_dir("round-trip");
    let project = common::project();
    project.save(&dir).expect("save");
    assert_eq!(Project::load(&dir).expect("load"), project);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn load_refuses_an_invalid_project() {
    let dir = common::temp_project_dir("invalid");
    let mut project = common::project();
    project.tracks[0].clips[0].asset = common::asset_id("does-not-exist");
    project
        .save(&dir)
        .expect("save is allowed even when invalid");

    let error = Project::load(&dir).expect_err("load must validate");
    assert!(matches!(error, LoadError::Invalid(_)), "got {error:?}");
    std::fs::remove_dir_all(&dir).ok();
}
