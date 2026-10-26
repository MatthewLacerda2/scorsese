//! `set`, from the outside: each field reaches the number it names, and the
//! values a field cannot hold are refused before anything is produced. The
//! routing between neighbours is `tune.rs`'s own tests; these are the edges.

use scorsese_providers::synth::{Change, Setting, set};

const SONG: &str = r#"{
    "recipe": "song", "bpm": 96.0, "seed": 1,
    "tracks": [
        { "name": "lead", "gain": 0.9,
          "patch": { "source": { "kind": "noise" },
                     "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } },
        { "name": "bass", "gain": 0.5,
          "patch": { "source": { "kind": "noise" },
                     "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } }
    ],
    "patterns": { "a": { "beats": 4.0, "notes": [
        { "track": "lead", "note": "C3", "start": 0.0, "dur": 1.0 } ] } },
    "arrangement": ["a"]
}"#;

const PATCH: &str = r#"{
    "recipe": "patch", "note": "C4", "duration": 0.4, "velocity": 1.0, "seed": 0,
    "patch": { "source": { "kind": "noise" },
               "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } }
}"#;

fn tune(json: &str, field: &str, track: Option<&str>, value: f64) -> Result<Change, String> {
    set(
        json,
        &Setting {
            field,
            track,
            value,
        },
    )
}

/// A track is found by its name: the first track is not a stand-in for it.
#[test]
fn gain_moves_the_track_it_names_and_no_other() {
    let change = tune(SONG, "gain", Some("bass"), 0.25).expect("settable");
    assert_eq!((change.was.as_str(), change.kind), ("0.5", "song"));
    assert!(
        change.document.contains("\"gain\": 0.25"),
        "{}",
        change.document
    );
    assert!(
        change.document.contains("\"gain\": 0.9"),
        "the lead kept its fader"
    );

    let refusal = tune(SONG, "gain", Some("drums"), 0.5).expect_err("no such track");
    assert!(refusal.contains("no track `drums`"), "{refusal}");
    assert!(
        refusal.contains("lead, bass"),
        "it names the ones there are: {refusal}"
    );
}

#[test]
fn every_song_and_patch_field_reaches_its_own_number() {
    let swing = tune(SONG, "swing", None, 0.3).expect("settable");
    assert!(
        swing.document.contains("\"swing\": 0.3"),
        "{}",
        swing.document
    );
    let seed = tune(SONG, "seed", None, 7.0).expect("settable");
    assert_eq!(seed.was, "1");
    assert!(seed.document.contains("\"seed\": 7"), "{}", seed.document);

    let duration = tune(PATCH, "duration", None, 1.5).expect("settable");
    assert_eq!((duration.was.as_str(), duration.kind), ("0.4", "patch"));
    assert!(duration.document.contains("\"duration\": 1.5"));
    let velocity = tune(PATCH, "velocity", None, 0.25).expect("settable");
    assert_eq!(velocity.was, "1");
    assert!(velocity.document.contains("\"velocity\": 0.25"));
    let seed = tune(PATCH, "seed", None, 12.0).expect("settable");
    assert!(seed.document.contains("\"seed\": 12"), "{}", seed.document);
}

/// A field the shape does not have is refused with the ones it does.
#[test]
fn an_unknown_field_is_refused_with_what_there_is() {
    let refusal = tune(SONG, "pitch", None, 1.0).expect_err("no such field");
    assert!(
        refusal.contains("a song recipe has no `pitch`"),
        "{refusal}"
    );
    assert!(refusal.contains("bpm, seed, swing"), "{refusal}");
    let refusal = tune(PATCH, "bpm", None, 1.0).expect_err("a patch has no tempo");
    assert!(refusal.contains("a patch recipe has no `bpm`"), "{refusal}");
    assert!(refusal.contains("duration, velocity"), "{refusal}");
}

#[test]
fn a_decimal_field_refuses_what_is_not_a_number() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let refusal = tune(SONG, "bpm", None, value).expect_err("not a number");
        assert!(
            refusal.contains("not a number a recipe can hold"),
            "{refusal}"
        );
    }
    assert_eq!(tune(SONG, "bpm", None, -5.0).expect("finite").was, "96");
}

/// A seed is a whole, non-negative number a JSON number can still name —
/// and both ends of that range are seeds.
#[test]
fn a_seed_is_a_whole_number_in_range_and_nothing_else() {
    const CEILING: f64 = 9_007_199_254_740_992.0;
    for value in [-1.0, 0.5, 2.5, f64::NAN, f64::INFINITY, CEILING * 2.0] {
        let refusal = tune(SONG, "seed", None, value).expect_err("not a seed");
        assert!(
            refusal.contains("a seed is a whole number"),
            "{value}: {refusal}"
        );
    }
    let zero = tune(SONG, "seed", None, 0.0).expect("zero is a seed");
    assert!(zero.document.contains("\"seed\": 0"), "{}", zero.document);
    let top = tune(SONG, "seed", None, CEILING).expect("the ceiling is a seed");
    assert!(
        top.document.contains("9007199254740992"),
        "{}",
        top.document
    );
}
