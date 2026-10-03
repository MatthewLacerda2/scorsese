//! A folder of frames brought in as one image sequence, and sequences made
//! and changed from stills already in the pool.

mod change;

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{
    AssetId, AssetKind, ImportError, MediaMetadata, ProbeError, ProbeMedia, SkipReason,
    import_sequence,
};

/// A folder named `spin` holding three frames, numbered so a plain sort of
/// names would play them wrong, and two files that are not frames.
pub(crate) fn folder(dir: &std::path::Path) -> std::path::PathBuf {
    for (name, bytes) in [
        ("frame_10.png", "ten"),
        ("frame_2.png", "two"),
        ("frame_1.png", "one"),
        ("notes.txt", "not a frame"),
        ("wiggle.gif", "an animation"),
    ] {
        source_file(dir, &format!("spin/{name}"), bytes.as_bytes());
    }
    dir.join("sources/spin")
}

#[test]
fn a_folder_of_frames_becomes_one_sequence_in_number_order() {
    let (dir, mut project) = new_project("sequence-import");
    let report =
        import_sequence(&mut project, &dir, &folder(&dir), &StubProbe::image()).expect("import");

    assert_eq!(report.id, AssetId::new("spin"));
    let ids: Vec<&str> = report.stills.iter().map(AssetId::as_str).collect();
    assert_eq!(ids, ["spin-frame_1", "spin-frame_2", "spin-frame_10"]);
    assert!(
        dir.join("assets/spin/frame_10.png").is_file(),
        "one folder on disk"
    );
    let skipped: Vec<SkipReason> = report.skipped.iter().map(|s| s.why).collect();
    assert_eq!(skipped, [SkipReason::NotAFrame, SkipReason::NotAFrame]);
    assert_eq!(report.gaps.len(), 1, "3 to 9 are missing");
    assert_eq!(report.gaps[0].missing, 7);

    assert_eq!(report.reused, 0);
    let first = project.asset(&report.stills[0]).expect("a still");
    assert_eq!(
        first.media.and_then(|m| m.width),
        Some(64),
        "each still is probed"
    );
    let sequence = project.asset(&report.id).expect("added");
    assert_eq!(sequence.kind, AssetKind::ImageSequence);
    let played = &sequence.sequence.as_ref().expect("its block").stills;
    assert_eq!(played, &report.stills);
    project.validate().expect("and the project validates");
}

#[test]
fn importing_the_same_folder_again_adds_nothing() {
    let (dir, mut project) = new_project("sequence-again");
    let source = folder(&dir);
    let first = import_sequence(&mut project, &dir, &source, &StubProbe::image()).expect("one");
    let count = project.assets.len();
    let again = import_sequence(&mut project, &dir, &source, &StubProbe::image()).expect("two");
    assert!(again.existed);
    assert_eq!(again.id, first.id);
    assert_eq!(again.reused, 3, "every still was already in the pool");
    assert_eq!(project.assets.len(), count);
}

#[test]
fn a_frame_the_pool_already_holds_is_reused_and_counted() {
    let (dir, mut project) = new_project("sequence-partly");
    let source = folder(&dir);
    let one = source_file(&dir, "loose/one.png", b"one");
    scorsese_core::import_asset(&mut project, &dir, &one, None, &StubProbe::image())
        .expect("one frame already in the pool");
    let report = import_sequence(&mut project, &dir, &source, &StubProbe::image()).expect("import");
    assert_eq!(report.reused, 1);
    assert_eq!(
        report.stills[0].as_str(),
        "one",
        "the asset that already held it"
    );
}

#[test]
fn frames_of_two_formats_are_refused_with_nothing_copied() {
    let (dir, mut project) = new_project("sequence-mixed");
    let source = folder(&dir);
    source_file(&dir, "spin/frame_3.jpg", b"a jpeg");
    let error =
        import_sequence(&mut project, &dir, &source, &StubProbe::image()).expect_err("two formats");
    assert!(matches!(error, ImportError::NotASequence { .. }), "{error}");
    assert!(!dir.join("assets/spin").exists());
    assert!(project.assets.is_empty());
}

#[test]
fn frames_of_two_sizes_are_refused_with_nothing_copied() {
    let (dir, mut project) = new_project("sequence-sizes");
    source_file(&dir, "two/a_1.png", b"one");
    source_file(&dir, "two/a_2.png", b"two");
    let sizes = std::cell::Cell::new(0);
    let probe = SizedByCall(&sizes);
    let error = import_sequence(&mut project, &dir, &dir.join("sources/two"), &probe)
        .expect_err("two sizes");
    assert!(matches!(error, ImportError::NotASequence { .. }), "{error}");
    assert!(project.assets.is_empty());
}

/// Answers 64x64 the first time and 32x32 after.
struct SizedByCall<'a>(&'a std::cell::Cell<u32>);

impl ProbeMedia for SizedByCall<'_> {
    fn probe(&self, _: &std::path::Path) -> Result<MediaMetadata, ProbeError> {
        let side = if self.0.replace(self.0.get() + 1) == 0 {
            64
        } else {
            32
        };
        Ok(MediaMetadata {
            width: Some(side),
            height: Some(side),
            ..MediaMetadata::default()
        })
    }
}
