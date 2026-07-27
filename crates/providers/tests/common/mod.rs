//! A real project directory in a temporary directory.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, ProjectPath};

/// A fresh `*.scor` directory with an empty project in it.
pub(crate) fn project(label: &str) -> (PathBuf, Project) {
    let dir = scratch(label);
    let project = Project::create(&dir, label, Fps::THIRTY).expect("create the project");
    (dir, project)
}

/// Writes a file into the project at a project-relative path, creating
/// whatever directories it needs.
pub(crate) fn write(dir: &Path, relative: &str, contents: &str) {
    let path = ProjectPath::new(relative).resolve(dir);
    let parent = path.parent().expect("a file has a parent directory");
    std::fs::create_dir_all(parent).expect("create the directory");
    std::fs::write(&path, contents).expect("write the file");
}

/// Adds a `synth_audio` asset pointing at `recipe`, and returns its id.
pub(crate) fn synth_asset(project: &mut Project, id: &str, recipe: &str) -> AssetId {
    let id = AssetId::new(id);
    project.assets.push(Asset::synth(
        id.clone(),
        AssetKind::SynthAudio,
        ProjectPath::new(recipe),
    ));
    id
}

/// A directory in the temporary area that nothing has created yet.
pub(crate) fn scratch(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-providers-{label}-{}-{unique}.scor",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}
