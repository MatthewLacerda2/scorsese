//! The one number that softens a clip, and what a document says about it.
//!
//! `blur` arrived in schema 25 as a field with a skip-predicate on it, and the
//! predicate is the whole of the compatibility story: absent means sharp, sharp
//! writes nothing, so a project written before the field existed comes back out
//! of a load-and-save exactly as it went in rather than gaining a key nobody
//! asked for. That claim is only true while something asserts it, which is what
//! this file is for.

mod common;

use scorsese_core::{AssetId, Clip, ClipId, Frames, Project};

/// The fixture — which mentions no blur anywhere — with its first clip softened
/// by `blur`.
fn softened(blur: f64) -> Project {
    let mut project = common::project();
    project
        .tracks
        .first_mut()
        .expect("a first track")
        .clips
        .first_mut()
        .expect("a first clip")
        .blur = blur;
    project
}

/// A clip nobody has said anything about is sharp, so the document says nothing
/// about it either.
#[test]
fn a_sharp_clip_writes_no_blur_at_all() {
    let clip = Clip::new(
        ClipId::new("c1"),
        AssetId::new("a1"),
        Frames::ZERO,
        Frames(120),
    );
    assert_eq!(clip.blur, 0.0, "a new clip is as sharp as its source");

    let json = common::project().to_json().expect("serialise");
    assert!(
        !json.contains("blur"),
        "the fixture softens nothing: {json}"
    );

    // And a zero written out by hand is the same document, which is what makes
    // the round trip a fixed point rather than a slow drift: saving a project
    // twice cannot add the key on the second pass.
    let json = softened(0.0).to_json().expect("serialise");
    assert!(
        !json.contains("blur"),
        "a blur of exactly zero is an absent blur: {json}"
    );
}

/// The other half, and the one a bug would hide behind: a predicate that skipped
/// *everything* would also produce a document with no `blur` in it, and the
/// clip would silently render sharp on the next load.
#[test]
fn a_softened_clip_writes_it_and_reads_it_back() {
    let project = softened(0.012);
    let json = project.to_json().expect("serialise");
    assert!(
        json.contains("\"blur\": 0.012"),
        "a blur somebody chose is in the document: {json}"
    );
    assert_eq!(
        Project::from_json(&json).expect("reparse"),
        project,
        "and survives the save"
    );
    assert_eq!(project.validate(), Ok(()));
}

/// Absent is not a third state. It is zero, decided here so that nothing
/// downstream has to ask what a missing `blur` means.
#[test]
fn an_absent_blur_parses_as_sharp() {
    let json = common::document(
        r#""assets": [{ "id": "a", "kind": "image", "path": "assets/a.png" }],
           "tracks": [{ "id": "v1", "kind": "video", "clips": [
               { "id": "c", "asset": "a", "start": 0, "duration": 30 }] }]"#,
    );
    let project = Project::from_json(&json).expect("parses");
    let (_, clip) = project.clips().next().expect("a clip");
    assert_eq!(clip.blur, 0.0);
}

/// Exactly zero and not "at or below zero", which is the distinction the
/// predicate's doc comment makes and the one a comparison mutated the other way
/// would lose. A negative number is not a blur and the compositor treats it as
/// none — but dropping it on the way past would be this crate quietly editing
/// somebody's document, and the number they wrote would be gone.
#[test]
fn a_negative_blur_is_kept_rather_than_dropped() {
    let project = softened(-1.0);
    let json = project.to_json().expect("serialise");
    assert!(
        json.contains("\"blur\": -1.0"),
        "a number that softens nothing is still the number in the file: {json}"
    );
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
}
