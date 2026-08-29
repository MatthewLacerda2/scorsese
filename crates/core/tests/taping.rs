//! The `vhs` field, and what a document says about it.
//!
//! A composite: five numbers and a mode under one key, arriving in schema 32.
//! What makes it worth its own file rather than a line in `optics.rs` is that
//! its skip predicate cannot be "is it zero" — **`mono` alone is a change**,
//! with every number at its neutral, so a predicate that only looked at the
//! numbers would drop a mode somebody set and the clip would come back in
//! colour. That is the claim these tests exist to hold.

mod common;

use scorsese_core::{AssetId, Clip, ClipId, Frames, Project, Vhs};

/// The fixture — which mentions `vhs` nowhere — with one on its first clip.
fn with(vhs: Vhs) -> Project {
    let mut project = common::project();
    project
        .tracks
        .first_mut()
        .expect("a first track")
        .clips
        .first_mut()
        .expect("a first clip")
        .vhs = vhs;
    project
}

/// A clip nobody has taped is neutral, so the document says nothing about it.
#[test]
fn an_untaped_clip_writes_nothing_at_all() {
    let clip = Clip::new(
        ClipId::new("c1"),
        AssetId::new("a1"),
        Frames::ZERO,
        Frames(120),
    );
    assert_eq!(clip.vhs, Vhs::NONE);
    assert!(clip.vhs.is_none());
    for json in [
        common::project().to_json().expect("serialise"),
        with(Vhs::NONE).to_json().expect("serialise"),
    ] {
        assert!(!json.contains("vhs"), "no tape is no key: {json}");
    }
}

/// The other half, and the one a predicate mutated to always-skip would hide
/// behind: a tape somebody wrote is in the file and comes back out of it.
#[test]
fn a_tape_somebody_chose_is_written_and_read_back() {
    let project = with(Vhs {
        chroma_bleed: 0.4,
        noise: 0.2,
        scanlines: 0.3,
        jitter: 0.1,
        head_switch: 0.25,
        mono: false,
    });
    let json = project.to_json().expect("serialise");
    for (key, value) in [
        ("chroma_bleed", "0.4"),
        ("noise", "0.2"),
        ("scanlines", "0.3"),
        ("jitter", "0.1"),
        ("head_switch", "0.25"),
    ] {
        assert!(
            json.contains(&format!("\"{key}\": {value}")),
            "`{key}` is in the document: {json}"
        );
    }
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
    assert_eq!(project.validate(), Ok(()));
}

/// **`mono` alone is not nothing.** Every number is neutral and the picture
/// still changes, because taking the colour out is a change — so this is the
/// one case a "are all the numbers zero" predicate gets wrong, and it would get
/// it wrong silently, on save, in somebody's project.
#[test]
fn a_mono_tape_with_no_numbers_on_it_still_survives_a_save() {
    let project = with(Vhs {
        mono: true,
        ..Vhs::NONE
    });
    let json = project.to_json().expect("serialise");
    assert!(json.contains("\"mono\": true"), "{json}");
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
}

/// Absent is not a third state, and neither is an empty object: both are the
/// tape that does nothing, decided here so nothing downstream has to ask.
#[test]
fn an_absent_tape_and_an_empty_one_are_the_same_tape() {
    let clips = |vhs: &str| {
        format!(
            r#""assets": [{{ "id": "a", "kind": "image", "path": "assets/a.png" }}],
               "tracks": [{{ "id": "v1", "kind": "video", "clips": [
                   {{ "id": "c", "asset": "a", "start": 0, "duration": 30{vhs} }}] }}]"#
        )
    };
    for body in [clips(""), clips(r#", "vhs": {}"#)] {
        let project = Project::from_json(&common::document(&body)).expect("parses");
        let (_, clip) = project.clips().next().expect("a clip");
        assert_eq!(clip.vhs, Vhs::NONE);
        // And it writes back out as nothing, so a load-and-save is a fixed
        // point rather than a document that grows a key on every pass.
        assert!(!project.to_json().expect("serialise").contains("vhs"));
    }
}

/// A misspelt sub-value is refused rather than ignored. Five numbers under one
/// key is exactly the shape where a typo would otherwise sit in the file being
/// silently neutral, which is the failure `deny_unknown_fields` exists for.
#[test]
fn a_sub_value_nobody_publishes_is_refused() {
    let body = r#""assets": [{ "id": "a", "kind": "image", "path": "assets/a.png" }],
           "tracks": [{ "id": "v1", "kind": "video", "clips": [
               { "id": "c", "asset": "a", "start": 0, "duration": 30,
                 "vhs": { "scanline": 0.4 } }] }]"#;
    let error = Project::from_json(&common::document(body)).expect_err("refused");
    assert!(format!("{error}").contains("scanline"), "{error}");
}
