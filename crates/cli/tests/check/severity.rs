//! The health→severity table, asset by asset.
//!
//! Every row is written twice on purpose — once referenced, once not. The
//! referenced/unreferenced axis is half the rule, and a table that only ever
//! saw one side of it would pass while ignoring `clip_count` entirely.

use scorsese_cli::commands::check::media::{Severity, findings};
use scorsese_core::{AssetHealth, AssetId, AssetKind, AssetStatus, GenerationState};

/// The severity `check` gives this health, or `None` when it says nothing.
fn severity_of(health: AssetHealth, clip_count: usize) -> Option<Severity> {
    findings(&[row(health, clip_count)])
        .first()
        .map(|finding| finding.severity)
}

fn row(health: AssetHealth, clip_count: usize) -> AssetStatus {
    AssetStatus {
        id: AssetId::new("subject"),
        kind: AssetKind::Video,
        health,
        clip_count,
    }
}

fn mismatch() -> AssetHealth {
    AssetHealth::HashMismatch {
        recorded: "a".repeat(64),
        found: "b".repeat(64),
    }
}

// A file a clip needs and cannot get is the whole reason this exists.

#[test]
fn a_referenced_missing_file_is_a_problem() {
    assert_eq!(
        severity_of(AssetHealth::Missing, 1),
        Some(Severity::Problem)
    );
}

#[test]
fn a_referenced_unreadable_file_is_a_problem() {
    let health = AssetHealth::Unreadable("permission denied".to_owned());
    assert_eq!(severity_of(health, 1), Some(Severity::Problem));
}

// These render. They are worth saying and not worth failing over.

#[test]
fn a_referenced_changed_file_is_a_warning() {
    assert_eq!(severity_of(mismatch(), 1), Some(Severity::Warning));
}

#[test]
fn a_referenced_unprobed_file_is_a_warning() {
    assert_eq!(
        severity_of(AssetHealth::Unprobed, 1),
        Some(Severity::Warning)
    );
}

// Silence. A healthy pool and a pre-GO sketch must read identically clean.

#[test]
fn healthy_media_says_nothing() {
    assert_eq!(severity_of(AssetHealth::Ok, 1), None);
}

#[test]
fn inline_text_says_nothing() {
    assert_eq!(severity_of(AssetHealth::Inline, 1), None);
}

#[test]
fn a_prompt_awaiting_generation_says_nothing() {
    for state in [
        GenerationState::Sketch,
        GenerationState::Queued,
        GenerationState::Stale,
    ] {
        assert_eq!(
            severity_of(AssetHealth::Awaiting(state), 1),
            None,
            "{state:?} is the normal state of a project before GO"
        );
    }
}

// Nothing an unreferenced asset does can break a render, so it drops a step.

#[test]
fn an_unreferenced_missing_file_is_only_a_warning() {
    assert_eq!(
        severity_of(AssetHealth::Missing, 0),
        Some(Severity::Warning)
    );
}

#[test]
fn an_unreferenced_unreadable_file_is_only_a_warning() {
    let health = AssetHealth::Unreadable("permission denied".to_owned());
    assert_eq!(severity_of(health, 0), Some(Severity::Warning));
}

#[test]
fn an_unreferenced_changed_or_unprobed_file_says_nothing() {
    assert_eq!(severity_of(mismatch(), 0), None);
    assert_eq!(severity_of(AssetHealth::Unprobed, 0), None);
}

#[test]
fn a_finding_names_the_asset_and_how_many_clips_want_it() {
    let found = findings(&[row(AssetHealth::Missing, 2)]);
    let line = found
        .first()
        .expect("a missing file is reported")
        .to_string();
    assert!(line.contains("subject"), "{line}");
    assert!(line.contains("2 clips use it"), "{line}");
}

#[test]
fn an_unreferenced_finding_points_at_gc() {
    let found = findings(&[row(AssetHealth::Missing, 0)]);
    let line = found
        .first()
        .expect("a missing file is reported")
        .to_string();
    assert!(line.contains("gc"), "{line}");
}
