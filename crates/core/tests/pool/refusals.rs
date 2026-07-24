//! Files that must not make it into the pool, and what is left behind when
//! one is rejected.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{AssetKind, ImportError, import_asset};

#[test]
fn an_unknown_extension_asks_rather_than_guesses() {
    let (dir, mut project) = new_project("refuse-unknown");
    let source = source_file(&dir, "mystery.xyz", b"who knows");
    let error = import_asset(&mut project, &dir, &source, None, &StubProbe::video())
        .expect_err("must not guess");
    assert!(
        matches!(error, ImportError::UnknownKind { .. }),
        "got {error:?}"
    );
}

#[test]
fn a_prompt_kind_cannot_be_imported() {
    // A Veo or ElevenLabs asset is authored as a prompt; there is no file to
    // copy in until GO has run.
    let (dir, mut project) = new_project("refuse-prompt");
    let source = source_file(&dir, "clip.mp4", b"video");
    let error = import_asset(
        &mut project,
        &dir,
        &source,
        Some(AssetKind::GeneratedVideo),
        &StubProbe::video(),
    )
    .expect_err("prompts are authored, not imported");
    assert!(
        matches!(error, ImportError::NotImportable { .. }),
        "got {error:?}"
    );
}

#[test]
fn a_file_whose_extension_lies_is_caught() {
    let (dir, mut project) = new_project("refuse-lying");
    let source = source_file(&dir, "notreally.mp4", b"actually audio");
    let error = import_asset(&mut project, &dir, &source, None, &StubProbe::audio())
        .expect_err("a video with no picture is not a video");
    assert!(
        matches!(error, ImportError::KindMismatch { .. }),
        "got {error:?}"
    );
}

#[test]
fn a_failed_import_leaves_the_project_untouched() {
    let (dir, mut project) = new_project("refuse-clean");
    let source = source_file(&dir, "broken.mp4", b"garbage");

    let error = import_asset(
        &mut project,
        &dir,
        &source,
        None,
        &StubProbe::failing("no such codec"),
    )
    .expect_err("probe failure fails the import");

    assert!(matches!(error, ImportError::Probe(_)), "got {error:?}");
    assert!(project.assets.is_empty(), "nothing was added to the table");
    let copied: Vec<_> = std::fs::read_dir(dir.join("assets"))
        .expect("assets dir")
        .flatten()
        .collect();
    assert!(copied.is_empty(), "no stray file was left behind");
}
