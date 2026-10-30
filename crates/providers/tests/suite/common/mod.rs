//! A real project directory in a temporary directory.

#[path = "../../../../../tools/scratch.rs"]
pub(crate) mod scratch;

use std::path::{Path, PathBuf};

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, ProjectPath};

/// A fresh `*.scor` directory with an empty project in it.
pub(crate) fn project(label: &str) -> (PathBuf, Project) {
    let dir = scratch(label);
    let project = Project::create(&dir, Some(label), Fps::THIRTY, scorsese_core::Ratio::WIDE)
        .expect("create the project");
    (dir, project)
}

/// Writes a file into the project at a project-relative path, creating
/// whatever directories it needs.
pub(crate) fn write(dir: &Path, relative: &str, contents: &str) {
    let path = ProjectPath::new(relative)
        .resolve(dir)
        .expect("a project path");
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
/// Removed when the test ends.
pub(crate) fn scratch(label: &str) -> PathBuf {
    scratch::fresh(&format!("scorsese-providers-{label}"), ".scor")
}
