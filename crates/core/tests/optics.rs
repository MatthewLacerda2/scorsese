//! The two numbers that sit beside a grade rather than inside it, and what a
//! document says about them.
//!
//! `blur` arrived in schema 25 and `aberration` in schema 31, and both are a
//! field with a skip-predicate on it. The predicate is the whole of the
//! round-trip story: neutral means absent, absent writes nothing, so a project
//! that says nothing about either comes back out of a load-and-save exactly as
//! it went in rather than gaining a key nobody asked for. That claim is only
//! true while something asserts it, which is what this file is for.
//!
//! Both fields, one set of tests, because they are one shape: a bare `f64` on
//! a clip whose neutral is exactly zero. A second copy of these tests would
//! only assert that the copy agrees with the original.

mod common;

use scorsese_core::{AssetId, Clip, ClipId, Frames, Project};

/// A field of a clip that behaves this way, as a name and the two accessors a
/// test needs.
struct Number {
    key: &'static str,
    /// A value nobody would reach for by accident, so finding it in a document
    /// means it was written rather than defaulted.
    chosen: f64,
    of: fn(&Clip) -> f64,
    set: fn(&mut Clip, f64),
}

const NUMBERS: [Number; 2] = [
    Number {
        key: "blur",
        chosen: 0.012,
        of: |clip| clip.blur,
        set: |clip, value| clip.blur = value,
    },
    Number {
        key: "aberration",
        chosen: 0.0015,
        of: |clip| clip.aberration,
        set: |clip, value| clip.aberration = value,
    },
];

/// The fixture — which mentions neither field anywhere — with `value` on its
/// first clip.
fn with(number: &Number, value: f64) -> Project {
    let mut project = common::project();
    (number.set)(
        project
            .tracks
            .first_mut()
            .expect("a first track")
            .clips
            .first_mut()
            .expect("a first clip"),
        value,
    );
    project
}

/// A clip nobody has said anything about is neutral, so the document says
/// nothing about it either.
#[test]
fn a_neutral_clip_writes_nothing_at_all() {
    let clip = Clip::new(
        ClipId::new("c1"),
        AssetId::new("a1"),
        Frames::ZERO,
        Frames(120),
    );
    let fixture = common::project().to_json().expect("serialise");
    for number in &NUMBERS {
        assert_eq!(
            (number.of)(&clip),
            0.0,
            "a new clip is its source, `{}`",
            number.key
        );
        assert!(
            !fixture.contains(number.key),
            "the fixture asks for no `{}`: {fixture}",
            number.key
        );
        // And a zero written out by hand is the same document, which is what
        // makes the round trip a fixed point rather than a slow drift: saving a
        // project twice cannot add the key on the second pass.
        let json = with(number, 0.0).to_json().expect("serialise");
        assert!(
            !json.contains(number.key),
            "an exact zero is an absent `{}`: {json}",
            number.key
        );
    }
}

/// The other half, and the one a bug would hide behind: a predicate that
/// skipped *everything* would also produce a document with neither key in it,
/// and the clip would silently render neutral on the next load.
#[test]
fn a_number_somebody_chose_is_written_and_read_back() {
    for number in &NUMBERS {
        let project = with(number, number.chosen);
        let json = project.to_json().expect("serialise");
        assert!(
            json.contains(&format!("\"{}\": {}", number.key, number.chosen)),
            "a `{}` somebody chose is in the document: {json}",
            number.key
        );
        assert_eq!(
            Project::from_json(&json).expect("reparse"),
            project,
            "and `{}` survives the save",
            number.key
        );
        assert_eq!(project.validate(), Ok(()));
    }
}

/// Absent is not a third state. It is zero, decided here so that nothing
/// downstream has to ask what a missing field means.
#[test]
fn an_absent_number_parses_as_neutral() {
    let json = common::document(
        r#""assets": [{ "id": "a", "kind": "image", "path": "assets/a.png" }],
           "tracks": [{ "id": "v1", "kind": "video", "clips": [
               { "id": "c", "asset": "a", "start": 0, "duration": 30 }] }]"#,
    );
    let project = Project::from_json(&json).expect("parses");
    let (_, clip) = project.clips().next().expect("a clip");
    for number in &NUMBERS {
        assert_eq!((number.of)(clip), 0.0, "`{}`", number.key);
    }
}

/// Exactly zero and not "at or below zero", which is the distinction each
/// predicate's doc comment makes and the one a comparison mutated the other way
/// would lose. A negative number is neither a blur nor an aberration and the
/// compositor treats it as none — but dropping it on the way past would be this
/// crate quietly editing somebody's document, and the number they wrote would
/// be gone.
#[test]
fn a_negative_number_is_kept_rather_than_dropped() {
    for number in &NUMBERS {
        let project = with(number, -1.0);
        let json = project.to_json().expect("serialise");
        assert!(
            json.contains(&format!("\"{}\": -1.0", number.key)),
            "a number that does nothing is still the number in the file: {json}"
        );
        assert_eq!(Project::from_json(&json).expect("reparse"), project);
    }
}
