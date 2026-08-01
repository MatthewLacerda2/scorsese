//! `scorsese import` — media coming in from outside the project.
//!
//! Import is a **copy**, never a link, because the project directory has to
//! survive `scp -r`. So the assertions here are about two files: the one the
//! project now owns, and the one outside it that must be exactly as it was.

mod folders;
mod kinds;

use std::path::{Path, PathBuf};

use scorsese_core::{ASSETS_DIR, AssetKind};

use crate::common::media::{generate, tools};
use crate::common::{Run, new_project, reload, run_in, temp_dir};

/// A project, and a directory of media beside it that is not part of it.
pub(crate) struct Outside {
    pub(crate) project: PathBuf,
    pub(crate) elsewhere: PathBuf,
}

impl Outside {
    pub(crate) fn new(label: &str) -> Self {
        Self {
            project: new_project(label),
            elsewhere: temp_dir(&format!("{label}-sources")),
        }
    }

    /// A real, tiny media file outside the project, at `name`.
    pub(crate) fn media(&self, name: &str, source: &str) -> PathBuf {
        let file = self.elsewhere.join(name);
        generate(&tools(), &file, &["-f", "lavfi", "-i", source]);
        file
    }

    /// The same, of a source with no end to it — a still, which is one frame
    /// of something ffmpeg would otherwise go on producing forever.
    pub(crate) fn still(&self, name: &str) -> PathBuf {
        let file = self.elsewhere.join(name);
        generate(
            &tools(),
            &file,
            &[
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=64x64",
                "-frames:v",
                "1",
            ],
        );
        file
    }

    pub(crate) fn import(&self, file: &Path, arguments: &[&str]) -> Run {
        let mut all = vec!["import", file.to_str().expect("a utf-8 fixture path")];
        all.extend_from_slice(arguments);
        run_in(&self.project, &all)
    }

    /// Everything now sitting in the project's `assets/`.
    pub(crate) fn pool(&self) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(self.project.join(ASSETS_DIR))
            .expect("a project has an assets directory")
            .filter_map(|entry| Some(entry.ok()?.file_name().to_str()?.to_owned()))
            .collect();
        names.sort();
        names
    }

    pub(crate) fn clean(self) {
        std::fs::remove_dir_all(&self.project).ok();
        std::fs::remove_dir_all(&self.elsewhere).ok();
    }
}

/// A silent two-second clip, the shape most imports are.
pub(crate) const CLIP: &str = "color=c=red:s=64x64:d=2:r=30";

#[test]
fn an_imported_file_is_copied_in_probed_and_recorded() {
    let outside = Outside::new("import");
    let source = outside.media("boat.mp4", CLIP);
    outside.import(&source, &[]).ok();

    let project = reload(&outside.project);
    let [asset] = &project.assets[..] else {
        panic!("one import, one asset: {:?}", project.assets)
    };
    assert_eq!(asset.kind, AssetKind::Video);
    assert_eq!(
        asset.path.as_ref().map(ToString::to_string).as_deref(),
        Some("assets/boat.mp4")
    );
    assert!(asset.sha256.is_some(), "an import records what it hashed");
    let media = asset
        .media
        .as_ref()
        .expect("an import probes what it copied");
    assert_eq!((media.width, media.height), (Some(64), Some(64)));

    assert_eq!(outside.pool(), ["boat.mp4"]);
    assert!(
        source.is_file(),
        "the file outside the project is a copy source, not a move source"
    );
    outside.clean();
}

#[test]
fn importing_the_same_bytes_again_copies_nothing_and_adds_nothing() {
    // The thing an agent does by accident: an import loop re-run. Assets are
    // entities, and two clips pointing at one entity is the intended shape.
    let outside = Outside::new("reimport");
    let source = outside.media("boat.mp4", CLIP);
    outside.import(&source, &[]).ok();

    let same = outside.media("a-different-name.mp4", CLIP);
    let again = outside.import(&same, &[]).ok();
    again.says("already in the pool");

    assert_eq!(reload(&outside.project).assets.len(), 1);
    assert_eq!(outside.pool(), ["boat.mp4"], "nothing was copied twice");
    outside.clean();
}

#[test]
fn a_second_file_of_the_same_name_gets_a_name_of_its_own() {
    // Different bytes, same file name. Both belong to the project, so neither
    // may overwrite the other.
    let outside = Outside::new("collide");
    let first = outside.media("boat.mp4", CLIP);
    outside.import(&first, &[]).ok();
    let second = outside.media("other/boat.mp4", "color=c=blue:s=64x64:d=2:r=30");
    outside.import(&second, &[]).ok();

    assert_eq!(outside.pool(), ["boat-2.mp4", "boat.mp4"]);
    assert_eq!(reload(&outside.project).assets.len(), 2);
    outside.clean();
}
