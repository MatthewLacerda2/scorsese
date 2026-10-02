//! A bake that is asked to stop (#661): between recipes, and part way through
//! one — and in neither case is anything half-made left in `generated/`,
//! where it would be served as the finished bake for ever after.

use std::cell::Cell;
use std::path::Path;

use crate::common::{project, synth_asset, write};
use scorsese_providers::synth::{SynthesisError, bake_asset_unless, bake_pending_unless};

/// Two notes on an inline pluck: enough for a stop to land between them.
const TUNE: &str = r#"{
  "recipe": "song", "bpm": 120, "seed": 3,
  "tracks": [{ "name": "bass", "gain": 0.9, "patch": {
      "source": { "kind": "karplus", "damping": 0.99, "brightness": 0.4 },
      "amp": { "a": 0.001, "d": 0.2, "s": 0.0, "r": 0.1 } } }],
  "patterns": { "a": { "beats": 2, "notes": [
      { "track": "bass", "note": "E2", "start": 0, "dur": 0.5 },
      { "track": "bass", "note": "B2", "start": 1, "dur": 0.5 } ] } },
  "arrangement": ["a"]
}"#;

/// How many bakes are in `generated/`.
fn bakes(dir: &Path) -> usize {
    std::fs::read_dir(dir.join("generated")).map_or(0, |entries| entries.count())
}

#[test]
fn a_song_stopped_between_its_notes_leaves_nothing_behind() {
    let (dir, mut project) = project("stopped-mid-song");
    write(&dir, "recipes/tune.json", TUNE);
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");

    // Asked before each note: the second question comes after the first note
    // was rendered and before the second.
    let asked = Cell::new(0u32);
    let stop = || {
        asked.set(asked.get() + 1);
        asked.get() > 1
    };
    let stopped = bake_asset_unless(&mut project, &dir, &id, &stop).expect_err("stopped");
    assert!(matches!(stopped, SynthesisError::Stopped), "{stopped:?}");
    assert_eq!(bakes(&dir), 0, "no partial file at the bake's address");
    assert_eq!(asked.get(), 2, "stopped part way, not before it began");
    assert!(project.asset(&id).and_then(|it| it.path.as_ref()).is_none());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_pending_bake_stops_between_recipes_and_keeps_what_it_finished() {
    let (dir, mut project) = project("stopped-between");
    write(&dir, "recipes/one.json", TUNE);
    write(
        &dir,
        "recipes/two.json",
        &TUNE.replace("\"seed\": 3", "\"seed\": 4"),
    );
    synth_asset(&mut project, "one", "recipes/one.json");
    synth_asset(&mut project, "two", "recipes/two.json");

    // Stops the moment the first bake has landed.
    let stop = || bakes(&dir) > 0;
    let stopped = bake_pending_unless(&mut project, &dir, &stop).expect_err("stopped");
    assert!(matches!(stopped, SynthesisError::Stopped), "{stopped:?}");
    assert_eq!(
        bakes(&dir),
        1,
        "the finished one stays; the second never began"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_stop_already_asked_for_bakes_nothing() {
    let (dir, mut project) = project("stopped-before");
    write(&dir, "recipes/tune.json", TUNE);
    synth_asset(&mut project, "tune", "recipes/tune.json");

    let stopped = bake_pending_unless(&mut project, &dir, &|| true).expect_err("stopped");
    assert!(matches!(stopped, SynthesisError::Stopped), "{stopped:?}");
    assert_eq!(bakes(&dir), 0);
    std::fs::remove_dir_all(dir).ok();
}
