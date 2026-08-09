//! Importing media into the pool.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{AssetKind, Project, import_asset, import_path};

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

/// The suffix is right; being quiet about it was not. Two *different* files
/// that happen to share a name is the case — byte-identical content is deduped
/// before any of this and is the test below.
#[test]
fn a_suffixed_import_says_which_id_it_asked_for() {
    let (dir, mut project) = new_project("import-says-renamed");
    let first = source_file(&dir, "clip.mp4", b"first video");
    let second = source_file(&dir, "other/clip.mp4", b"second video");

    let one = import_path(&mut project, &dir, &first, None, &StubProbe::video()).expect("first");
    assert_eq!(one.imported[0].wanted, None, "it got the id it asked for");

    let two = import_path(&mut project, &dir, &second, None, &StubProbe::video()).expect("second");
    let two = &two.imported[0];
    assert_eq!(two.id.as_str(), "clip-2");
    assert_eq!(
        two.wanted.as_ref().map(|id| id.as_str()),
        Some("clip"),
        "the id it wanted, so the report can say the suffix fired"
    );
}

/// Content already in the pool comes back as the asset that holds it, whatever
/// that asset is called. That is dedup working, not a rename, and reporting it
/// as one would put a warning on the case an import loop hits every re-run.
#[test]
fn reusing_content_already_in_the_pool_is_not_a_rename() {
    let (dir, mut project) = new_project("import-reuse-not-rename");
    let first = source_file(&dir, "clip.mp4", b"identical bytes");
    let second = source_file(&dir, "copy-of-clip.mp4", b"identical bytes");

    import_path(&mut project, &dir, &first, None, &StubProbe::video()).expect("first");
    let report =
        import_path(&mut project, &dir, &second, None, &StubProbe::video()).expect("again");

    let one = &report.imported[0];
    assert!(one.reused, "same bytes, same asset");
    assert_eq!(one.id.as_str(), "clip");
    assert_eq!(one.wanted, None, "reuse is not a rename");
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
