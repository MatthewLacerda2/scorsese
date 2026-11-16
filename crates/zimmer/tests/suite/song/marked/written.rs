//! Writing a marker down, and what it is refused for.

use scorsese_zimmer::SynthError;
use scorsese_zimmer::song::ArrangementEntry;

use super::{marked, marker};

/// The marker reads as written and writes back as written: one key.
#[test]
fn a_marker_round_trips_as_one_key() {
    let entry: ArrangementEntry =
        serde_json::from_str(r#"{ "section": "verse" }"#).expect("parses");
    assert_eq!(entry, marker("verse"));
    assert_eq!(
        serde_json::to_string(&entry).expect("serialise"),
        r#"{"section":"verse"}"#
    );
}

/// A marker takes nothing else, and says which key it was given.
#[test]
fn a_marker_names_the_key_it_does_not_take() {
    for (written, key) in [
        (r#"{ "section": "verse", "layers": ["a"] }"#, "layers"),
        (r#"{ "section": "verse", "transpose": 2 }"#, "transpose"),
    ] {
        let error = serde_json::from_str::<ArrangementEntry>(written)
            .expect_err("refused")
            .to_string();
        assert!(error.contains(key), "`{written}` was refused as: {error}");
    }
}

/// A second `chorus` is refused, with the fix in the words.
#[test]
fn a_name_used_twice_is_refused_by_name() {
    let song = marked(&[(0, "chorus"), (3, "chorus")]);
    let error = song.validate().expect_err("refused");
    assert!(
        matches!(&error, SynthError::BadSectionName { name, taken: true } if name == "chorus"),
        "{error:?}"
    );
    assert!(error.to_string().contains(r#""chorus 2""#), "{error}");
}

/// A marker with nothing before the next one, or before the end, is a typo.
#[test]
fn an_empty_section_is_refused_by_name() {
    for (song, empty) in [
        (marked(&[(0, "intro"), (0, "verse")]), "intro"),
        (marked(&[(0, "intro"), (6, "outro")]), "outro"),
    ] {
        assert_eq!(
            song.validate(),
            Err(SynthError::EmptySection { name: empty.into() })
        );
    }
}

/// An unnamed marker names nothing a report could print.
#[test]
fn an_empty_name_is_refused() {
    assert!(matches!(
        marked(&[(0, " ")]).validate(),
        Err(SynthError::BadSectionName { taken: false, .. })
    ));
}
