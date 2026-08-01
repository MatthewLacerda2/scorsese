//! What decoded a set of references, recorded beside them.
//!
//! The record only ever speaks inside a failure, so these arrange failures and
//! read what it said. What it must never do is decide anything on its own, and
//! that is asserted here rather than assumed.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_golden::{Fixture, GoldenError, Mode, RECORD_FILE, run};
use scorsese_render::frames;
use scorsese_render::{Frame, Resolution, Tools};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
const WORKSPACE: &str = env!("CARGO_TARGET_TMPDIR");

/// A private copy of a committed fixture, safe to vandalise.
fn borrowed(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let target = Path::new(WORKSPACE)
        .join("borrowed")
        .join(format!("{label}-{}-{unique}", std::process::id()));
    let _ = std::fs::remove_dir_all(&target);
    copy_tree(&Path::new(FIXTURES).join("letterbox"), &target);
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

/// Paints frame 0's reference blue, so the fixture cannot help but fail.
fn vandalise(fixture: &Path) {
    let reference = Fixture::load(fixture).expect("loads").reference(0);
    let mut blue = Frame::black(Resolution::new(64, 64).expect("raster"));
    for pixel in blue.bytes_mut().chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 255, u8::MAX]);
    }
    frames::write_png(&Tools::discover().expect("ffmpeg"), &reference, &blue)
        .expect("overwrite the reference");
}

fn record_of(fixture: &Path) -> PathBuf {
    Fixture::load(fixture)
        .expect("loads")
        .expected()
        .join(RECORD_FILE)
}

fn failure_text(fixture: &Path) -> String {
    let error =
        run(fixture, Mode::Check, Path::new(WORKSPACE)).expect_err("a blue reference cannot pass");
    assert!(
        matches!(error, GoldenError::Mismatch { .. }),
        "expected a pixel mismatch, got {error}"
    );
    error.to_string()
}

#[test]
fn a_failure_names_the_ffmpeg_that_decoded_the_frames() {
    let fixture = borrowed("names-the-decoder");
    vandalise(&fixture);

    let running = Tools::discover()
        .expect("ffmpeg")
        .ffmpeg_version()
        .expect("a version");
    let text = failure_text(&fixture);

    assert!(
        text.contains(&format!("decoded by ffmpeg {running}")),
        "the report says what produced the frames it is complaining about: {text}"
    );
}

#[test]
fn a_different_decoder_is_named_as_something_to_rule_out() {
    let fixture = borrowed("different-decoder");
    std::fs::write(record_of(&fixture), "0.0.0-not-a-real-build\n").expect("write a record");
    vandalise(&fixture);

    let text = failure_text(&fixture);

    assert!(
        text.contains("recorded under ffmpeg 0.0.0-not-a-real-build"),
        "the report names the decoder the references were recorded under: {text}"
    );
    assert!(
        text.contains("rule out"),
        "and offers it as a thing to check rather than as a verdict: {text}"
    );
}

#[test]
fn a_missing_record_is_reported_as_unknown_rather_than_assumed_to_match() {
    let fixture = borrowed("no-record");
    std::fs::remove_file(record_of(&fixture)).expect("remove the record");
    vandalise(&fixture);

    let text = failure_text(&fixture);

    assert!(
        text.contains("no record of what produced them"),
        "an absent record says so instead of implying agreement: {text}"
    );
}

#[test]
fn a_decoder_difference_alone_never_fails_a_fixture() {
    // The design rests on this. Were the record a check, everyone whose
    // distribution ships a different ffmpeg could not run the suite at all —
    // a gate red for a cause its author cannot see in their own diff.
    let fixture = borrowed("difference-is-not-a-failure");
    std::fs::write(record_of(&fixture), "0.0.0-not-a-real-build\n").expect("write a record");

    run(&fixture, Mode::Check, Path::new(WORKSPACE))
        .expect("frames that match still pass under a decoder the record does not name");
}

#[test]
fn blessing_records_the_ffmpeg_that_produced_the_new_references() {
    let fixture = borrowed("bless-records");
    std::fs::write(record_of(&fixture), "0.0.0-stale\n").expect("write a stale record");

    run(&fixture, Mode::Bless, Path::new(WORKSPACE)).expect("blessing succeeds");

    let running = Tools::discover()
        .expect("ffmpeg")
        .ffmpeg_version()
        .expect("a version");
    let written = std::fs::read_to_string(record_of(&fixture)).expect("the record was rewritten");
    assert_eq!(
        written.trim(),
        running,
        "the record cannot survive the frames it describes being replaced"
    );
}
