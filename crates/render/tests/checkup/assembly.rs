//! Every source a checkup draws on, asserted to reach the report.
//!
//! One test per row of the table in issue #334: each of these was something
//! `scorsese check` said and `project_check` did not, and the reason it could
//! diverge was that no test held the assembly itself.

use std::path::PathBuf;

use scorsese_core::{
    Asset, AssetKind, Clip, Easing, FontChoice, Frames, HashCheck, Keyframe, KeyframeTrack,
    Project, ProjectPath, PropertyPath, TextStyle,
};
use scorsese_render::Checkup;
use scorsese_render::checkup::{Severity, Verdict};

use crate::common::{clip, file_asset, project, text_asset, video_track};

/// A project directory that is not there, so every file the document names is
/// missing. Nothing is created, and there is nothing to clean up.
fn nowhere() -> PathBuf {
    std::env::temp_dir().join("scorsese-checkup-no-such-project.scor")
}

fn checkup(project: &Project) -> Checkup {
    Checkup::of(project, &nowhere(), HashCheck::Skip)
}

/// What the checkup says at one severity, which is what a caller prints.
fn said(checkup: &Checkup, severity: Severity) -> Vec<String> {
    checkup
        .lines()
        .iter()
        .filter(|line| line.severity == severity)
        .map(|line| line.says.clone())
        .collect()
}

fn report(checkup: &Checkup) -> String {
    match checkup.verdict() {
        Verdict::Problems(report) => report,
        Verdict::Clear(words) => panic!("expected problems, got `{words}`"),
    }
}

/// A title set in `face`, whatever this build makes of that name.
fn titled(face: &str) -> Asset {
    Asset {
        style: Some(TextStyle {
            font: FontChoice::Named(face.to_owned()),
            ..TextStyle::default()
        }),
        ..text_asset("title")
    }
}

/// A clip animating `property`, whatever that turns out to mean.
fn animating(id: &str, asset: &str, property: &str) -> Clip {
    let mut clip = clip(id, asset, 0, 30);
    clip.keyframes.push(KeyframeTrack::new(
        PropertyPath::new(property),
        vec![Keyframe {
            t: Frames::ZERO,
            value: 1.0,
            easing: Easing::Linear,
        }],
    ));
    clip
}

#[test]
fn a_project_of_inline_text_has_nothing_to_say() {
    let project = project(
        vec![text_asset("title")],
        vec![video_track("v1", vec![clip("c1", "title", 0, 30)])],
    );
    let checkup = checkup(&project);
    assert_eq!(checkup.warnings(), 0, "{:?}", checkup.lines());
    assert_eq!(
        checkup.verdict(),
        Verdict::Clear("no problems, nothing to warn about".to_owned())
    );
}

#[test]
fn a_file_a_clip_needs_going_missing_is_a_problem() {
    // The gap this exists to close: the document is perfectly coherent, and
    // the render would die partway through anyway.
    let project = project(
        vec![file_asset("boat", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "boat", 0, 30)])],
    );
    let problems = said(&checkup(&project), Severity::Problem);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("boat"), "{problems:?}");
    assert!(problems[0].contains("is not there"), "{problems:?}");
}

#[test]
fn a_font_this_build_does_not_ship_is_a_problem() {
    let project = project(
        vec![titled("blackletter")],
        vec![video_track("v1", vec![clip("c1", "title", 0, 30)])],
    );
    let problems = said(&checkup(&project), Severity::Problem);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("blackletter"), "{problems:?}");
}

#[test]
fn a_keyframe_track_nobody_animates_is_a_warning() {
    let clips = vec![animating("c1", "title", "opactiy")];
    let project = project(vec![text_asset("title")], vec![video_track("v1", clips)]);
    let warnings = said(&checkup(&project), Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("opactiy"), "{warnings:?}");
}

#[test]
fn a_script_the_disk_does_not_have_is_a_warning() {
    // A warning and never a problem: the project renders exactly the same
    // film without it. Worth saying because it is the one part of an edit
    // that cannot be reconstructed by watching the result.
    let project = Project {
        script: Some(ProjectPath::new("script.md")),
        ..project(
            vec![text_asset("title")],
            vec![video_track("v1", vec![clip("c1", "title", 0, 30)])],
        )
    };
    let checkup = checkup(&project);
    let warnings = said(&checkup, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("script.md"), "{warnings:?}");
    assert_eq!(
        checkup.verdict(),
        Verdict::Clear("no problems, 1 warning(s)".to_owned())
    );
}

#[test]
fn the_document_and_the_media_are_reported_together() {
    // Everything at once, under one heading, is what makes one call the whole
    // repair job — and which half of the checkup noticed what is not
    // something the reader should have to know.
    let clips = vec![clip("c1", "boat", 0, 30), clip("c2", "ghost", 30, 30)];
    let project = project(
        vec![file_asset("boat", AssetKind::Video)],
        vec![video_track("v1", clips)],
    );
    let report = report(&checkup(&project));
    assert!(report.contains("2 problems"), "{report}");
    assert!(report.contains("ghost"), "{report}");
    assert!(report.contains("boat"), "{report}");
}
