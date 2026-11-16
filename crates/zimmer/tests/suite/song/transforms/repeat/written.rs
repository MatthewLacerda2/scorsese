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
