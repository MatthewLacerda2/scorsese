//! Proving the gate can fail.
//!
//! A harness that cannot fail is worse than no harness, because it reads as
//! coverage. These break a fixture on purpose and insist the harness notices,
//! says which frame, and leaves the evidence somewhere findable.
//!
//! They break the *reference* rather than the renderer, because the comparison
//! cannot tell the two apart — it holds two images against each other — and a
//! wrong reference is the half a test can arrange.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_golden::{Fixture, GoldenError, Mode, run};
use scorsese_render::frames;
use scorsese_render::{Frame, Resolution, Tools};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
const WORKSPACE: &str = env!("CARGO_TARGET_TMPDIR");

/// A private copy of a committed fixture, safe to vandalise.
fn borrowed(name: &str, label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let target = Path::new(WORKSPACE)
        .join("borrowed")
        .join(format!("{label}-{}-{unique}", std::process::id()));
    let _ = std::fs::remove_dir_all(&target);
    copy_tree(&Path::new(FIXTURES).join(name), &target);
    target
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create the copy");
    for entry in std::fs::read_dir(from).expect("read the fixture").flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy a fixture file");
        }
    }
}

fn outcome_of(fixture: &Path, mode: Mode) -> Result<scorsese_golden::Outcome, GoldenError> {
    run(fixture, mode, Path::new(WORKSPACE))
}

#[test]
fn a_wrong_reference_fails_and_leaves_the_frame_to_look_at() {
    let fixture = borrowed("letterbox", "wrong-reference");
    let reference = Fixture::load(&fixture).expect("loads").reference(0);
    let mut blue = Frame::black(Resolution::new(64, 64).expect("raster"));
    for pixel in blue.bytes_mut().chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 255, u8::MAX]);
    }
    frames::write_png(&Tools::discover().expect("ffmpeg"), &reference, &blue)
        .expect("overwrite the reference");

    let error = outcome_of(&fixture, Mode::Check).expect_err("a blue reference cannot pass");

    let GoldenError::Mismatch { mismatches, .. } = &error else {
        panic!("expected a mismatch, got {error}");
    };
    assert_eq!(mismatches.frames.len(), 1, "only frame 0 was vandalised");
    let failure = &mismatches.frames[0];
    assert_eq!(failure.frame, 0);
    assert!(
        failure.actual.is_file(),
        "the produced frame was written out"
    );
    assert!(
        mismatches.render.is_file(),
        "the render is kept for inspection when it fails"
    );
    assert!(
        error.to_string().contains("UPDATE_GOLDENS=1"),
        "the failure says how to re-bless: {error}"
    );
}

#[test]
fn a_missing_reference_fails_instead_of_quietly_passing() {
    let fixture = borrowed("letterbox", "missing-reference");
    let reference = Fixture::load(&fixture).expect("loads").reference(7);
    std::fs::remove_file(&reference).expect("remove a reference");

    let error = outcome_of(&fixture, Mode::Check).expect_err("nothing to compare is not a pass");
    assert!(
        matches!(error, GoldenError::NoReference { frame: 7, .. }),
        "got {error}"
    );
}

#[test]
fn blessing_writes_the_reference_back() {
    let fixture = borrowed("letterbox", "bless");
    let reference = Fixture::load(&fixture).expect("loads").reference(7);
    std::fs::remove_file(&reference).expect("remove a reference");

    let outcome = outcome_of(&fixture, Mode::Bless).expect("blessing succeeds");

    assert_eq!(outcome.blessed, vec![0, 7, 14], "every frame is rewritten");
    assert!(reference.is_file(), "the reference is back");
    outcome_of(&fixture, Mode::Check).expect("and now it checks out");
}

#[test]
fn a_fixture_comparing_a_frame_past_the_end_says_so() {
    // Not a regression — a fixture that would silently assert nothing.
    let fixture = borrowed("letterbox", "beyond-end");
    let manifest = fixture.join("fixture.json");
    let text = std::fs::read_to_string(&manifest).expect("read the manifest");
    std::fs::write(&manifest, text.replace("[0, 7, 14]", "[0, 7, 14, 900]"))
        .expect("write the manifest");

    let error = outcome_of(&fixture, Mode::Check).expect_err("frame 900 does not exist");
    assert!(
        matches!(
            error,
            GoldenError::FrameBeyondRender {
                frame: 900,
                frames: 15,
                ..
            }
        ),
        "got {error}"
    );
}
