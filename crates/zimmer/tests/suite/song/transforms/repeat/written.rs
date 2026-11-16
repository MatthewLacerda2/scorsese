//! Writing a repeating layer down, and what it is refused for.

use scorsese_zimmer::song::{ArrangementEntry, Play};
use scorsese_zimmer::{Song, SynthError};

use super::super::setup::play;
use super::{band, layered, repeating, written_once};

/// Nothing says how long an entry is when every layer of it repeats.
#[test]
fn an_entry_whose_every_layer_repeats_is_refused() {
    let mut endless = band();
    endless.arrangement = vec![
        "verse".into(),
        layered(vec![repeating("verse"), repeating("solo")]),
    ];
    assert_eq!(
        endless.validate(),
        Err(SynthError::EveryLayerRepeats { index: 1 })
    );
    let said = SynthError::EveryLayerRepeats { index: 1 }.to_string();
    assert!(said.contains("`arrangement[1]`"), "{said}");
}

/// A single entry has nothing to repeat against.
#[test]
fn repeat_on_a_single_entry_is_refused() {
    let mut alone = band();
    alone.arrangement = vec![repeating("verse").into()];
    assert_eq!(
        alone.validate(),
        Err(SynthError::RepeatAlone {
            index: 0,
            pattern: "verse".to_owned(),
        })
    );
}

/// What an agent writes reads back as it was written, and a layer that does
/// not repeat writes no `repeat` key at all.
#[test]
fn repeat_is_written_only_where_it_is_set() {
    let entry: ArrangementEntry =
        serde_json::from_str(r#"{ "layers": ["solo", { "pattern": "verse", "repeat": true }] }"#)
            .expect("parses");
    assert_eq!(entry, layered(vec!["solo".into(), repeating("verse")]));
    let once = written_once();
    assert_eq!(
        Song::from_json(&once.to_json().expect("serialise")).expect("parses"),
        once
    );

    let plain = serde_json::to_string(&Play {
        transpose: Some(3.0),
        ..play("verse")
    })
    .expect("serialise");
    assert!(!plain.contains("repeat"), "{plain}");
}

/// A repeat laid out more times than a slot may hold is refused before any
/// pass is counted out: a tiny document must not be unbounded work.
#[test]
fn a_repeat_with_too_many_passes_is_refused() {
    let mut endless = band();
    endless
        .patterns
        .get_mut("solo")
        .expect("defined above")
        .beats = 1.0e9;
    endless.arrangement = vec![layered(vec!["solo".into(), repeating("verse")])];
    let refused = endless.validate().expect_err("refused");
    let SynthError::TooManyPasses { index, pattern, .. } = &refused else {
        panic!("refused for the wrong reason: {refused:?}")
    };
    assert_eq!((*index, pattern.as_str()), (0, "verse"));
    assert!(refused.to_string().contains("`verse`"), "{refused}");
}

/// The limit is a count of passes, and the one at it plays: 1024 passes of
/// two beats fill a slot of 2048, and one more beat is refused. At 240 bpm,
/// so the slot is inside the longest a song may be.
#[test]
fn a_slot_holds_exactly_the_most_passes() {
    let filled = |slot: f32| {
        let mut song = band();
        song.bpm = 240.0;
        song.patterns.get_mut("solo").expect("defined above").beats = slot;
        song.arrangement = vec![layered(vec!["solo".into(), repeating("verse")])];
        song.validate()
    };
    assert_eq!(filled(2048.0), Ok(()));
    assert!(matches!(
        filled(2049.0),
        Err(SynthError::TooManyPasses { limit: 1024, .. })
    ));
}

/// A repeating pattern of no length is a bad length, said as one — not a
/// count of passes it cannot have.
#[test]
fn a_repeating_pattern_of_no_length_is_refused_for_its_length() {
    let mut empty = band();
    empty
        .patterns
        .get_mut("verse")
        .expect("the fixture's")
        .beats = 0.0;
    empty.arrangement = vec![layered(vec!["solo".into(), repeating("verse")])];
    assert!(matches!(
        empty.validate(),
        Err(SynthError::BadPatternBeats { .. })
    ));
}
