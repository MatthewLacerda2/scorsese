//! Shared fixtures. Each test file uses a different slice of this, so unused
//! items here are expected rather than dead.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_core::{
    Asset, AssetId, ClipId, Fps, Project, SCHEMA_VERSION, TrackId, ValidationError,
    ValidationErrors,
};

pub(crate) mod stub_probe;

/// A hand-written project exercising every asset kind, both track kinds, and
/// keyframes. Doubles as the worked example in `docs/project-format.md`.
pub(crate) const FIXTURE: &str = include_str!("../fixtures/narrated_teaser.json");

/// The fixture, parsed. Valid by construction — tests mutate a copy of it to
/// produce exactly one problem at a time.
pub(crate) fn project() -> Project {
    Project::from_json(FIXTURE).expect("fixture parses")
}

/// A minimal well-formed document with `body` spliced in — for the parse
/// failures that are about one field rather than a whole project.
pub(crate) fn document(body: &str) -> String {
    format!(
        r#"{{ "schema_version": {SCHEMA_VERSION}, "name": "probe",
              "timeline_fps": {{ "num": 30, "den": 1 }}, {body} }}"#
    )
}

pub(crate) fn asset_id(id: &str) -> AssetId {
    AssetId::new(id)
}

pub(crate) fn clip_id(id: &str) -> ClipId {
    ClipId::new(id)
}

pub(crate) fn track_id(id: &str) -> TrackId {
    TrackId::new(id)
}

/// The asset with this id, mutably.
pub(crate) fn asset_mut<'a>(project: &'a mut Project, id: &str) -> &'a mut Asset {
    project
        .assets
        .iter_mut()
        .find(|a| a.id.as_str() == id)
        .expect("asset exists")
}

/// The problems a project reports, or an empty list when it is valid.
pub(crate) fn problems(project: &Project) -> Vec<ValidationError> {
    project
        .validate()
        .err()
        .map_or_else(Vec::new, ValidationErrors::into_vec)
}

/// True when this problem is among the ones the project reports — for the
/// mutations that produce several at once, where naming all of them would say
/// less than naming the one the test is about.
pub(crate) fn reports(project: &Project, expected: impl Into<ValidationError>) -> bool {
    problems(project).contains(&expected.into())
}

/// Asserts the project reports exactly this one problem — which keeps a test
/// honest about the mutation it made, rather than passing on a side effect.
///
/// Takes anything a problem converts from, so a test names the catalogue its
/// subject lives in — `AssetProblem` or `TimelineProblem` — rather than the
/// wrapper that collects both.
#[track_caller]
pub(crate) fn assert_only_problem(project: &Project, expected: impl Into<ValidationError>) {
    let found = problems(project);
    assert_eq!(
        found.as_slice(),
        std::slice::from_ref(&expected.into()),
        "wrong problems reported"
    );
}

/// A fresh, empty `*.scor` directory with a project already created in it.
pub(crate) fn new_project(label: &str) -> (PathBuf, Project) {
    let dir = temp_project_dir(label);
    let project = Project::create(&dir, label, Fps::THIRTY).expect("create project");
    (dir, project)
}

/// Writes a file of `bytes` outside any project, to import from.
pub(crate) fn source_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join("sources").join(name);
    let parent = path.parent().expect("a source file has a parent directory");
    std::fs::create_dir_all(parent).expect("create source dir");
    std::fs::write(&path, bytes).expect("write source file");
    path
}

/// A fresh empty directory under the system temp dir, unique per call.
pub(crate) fn temp_project_dir(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-{label}-{}-{unique}.scor",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp project dir");
    dir
}
