//! Importing media into the pool.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{AssetKind, Project, import_asset};

#[test]
fn import_copies_hashes_and_probes() {
    let (dir, mut project) = new_project("import-basic");
    let source = source_file(&dir, "clip.mp4", b"pretend this is a video");

    let id = import_asset(&mut project, &dir, &source, None, &StubProbe::video())
        .expect("import succeeds");

    let asset = project.asset(&id).expect("asset is in the table");
    assert_eq!(asset.kind, AssetKind::Video);
    assert_eq!(
        asset.path.as_ref().map(|p| p.as_str()),
        Some("assets/clip.mp4")
    );
    assert_eq!(asset.media.and_then(|m| m.width), Some(320));
    assert_eq!(asset.sha256.as_ref().map(String::len), Some(64));
    assert!(
        dir.join("assets/clip.mp4").is_file(),
        "the file was copied in"
    );
    assert!(source.is_file(), "the original is left alone");
}

#[test]
fn an_imported_project_still_validates() {
    // The point of every path rule: what import writes must load cleanly on
    // another machine.
    let (dir, mut project) = new_project("import-valid");
    let source = source_file(&dir, "clip.mp4", b"video");
    import_asset(&mut project, &dir, &source, None, &StubProbe::video()).expect("import");

    project.save(&dir).expect("save");
    let reloaded = Project::load(&dir).expect("load validates");
    assert_eq!(reloaded.assets.len(), 1);
}

#[test]
fn importing_the_same_content_twice_reuses_the_asset() {
    let (dir, mut project) = new_project("import-dedup");
    let first = source_file(&dir, "clip.mp4", b"identical bytes");
    let second = source_file(&dir, "copy-of-clip.mp4", b"identical bytes");

    let a = import_asset(&mut project, &dir, &first, None, &StubProbe::video()).expect("first");
    let b = import_asset(&mut project, &dir, &second, None, &StubProbe::video()).expect("second");

    assert_eq!(a, b, "same content is one asset");
    assert_eq!(project.assets.len(), 1);
    assert!(
        !dir.join("assets/copy-of-clip.mp4").exists(),
        "nothing was copied twice"
    );
}

#[test]
fn two_files_with_one_name_both_survive() {
    let (dir, mut project) = new_project("import-collision");
    let first = source_file(&dir, "clip.mp4", b"first video");
    let second = source_file(&dir, "other/clip.mp4", b"second video");

    import_asset(&mut project, &dir, &first, None, &StubProbe::video()).expect("first");
    let id = import_asset(&mut project, &dir, &second, None, &StubProbe::video()).expect("second");

    assert_eq!(id.as_str(), "clip-2");
    assert_eq!(
        project
            .asset(&id)
            .and_then(|a| a.path.as_ref())
            .map(|p| p.as_str()),
        Some("assets/clip-2.mp4")
    );
}

#[test]
fn a_still_image_keeps_no_frame_rate() {
    let (dir, mut project) = new_project("import-image");
    let source = source_file(&dir, "logo.png", b"pretend png");

    let id = import_asset(&mut project, &dir, &source, None, &StubProbe::image()).expect("import");

    let media = project.asset(&id).and_then(|a| a.media).expect("metadata");
    assert_eq!(media.width, Some(64));
    assert_eq!(media.frame_rate, None, "a still has no frame rate");
    assert_eq!(
        media.duration_seconds, None,
        "how long it shows is the clip's business"
    );
}

#[test]
fn the_extension_decides_the_kind_unless_told_otherwise() {
    let (dir, mut project) = new_project("import-kind");
    let wav = source_file(&dir, "beep.wav", b"audio");
    let id = import_asset(&mut project, &dir, &wav, None, &StubProbe::audio()).expect("import");
    assert_eq!(project.asset(&id).map(|a| a.kind), Some(AssetKind::Audio));
}
