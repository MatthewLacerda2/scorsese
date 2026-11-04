//! A `.scor` received from someone else may carry `cache` itself as a link
//! (#1718): a partial bake or a MIDI export kept in the cache writes nothing
//! where it leads.

#![cfg(unix)]

use crate::common::{project, scratch, synth_asset, write};
use scorsese_providers::synth::{Excerpt, bake_partial, export_midi};

const SONG: &str = r#"{
  "recipe": "song", "bpm": 100, "seed": 1,
  "tracks": [
    { "name": "lead", "patch": { "source": { "kind": "noise" },
        "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } }
  ],
  "patterns": { "a": { "beats": 2, "notes": [
      { "track": "lead", "note": "C4", "start": 0, "dur": 1 } ] } },
  "arrangement": ["a"]
}"#;

#[test]
fn a_partial_bake_or_a_midi_export_is_refused_through_a_linked_cache() {
    let (dir, mut project) = project("synth-linked-cache");
    let outside = scratch("synth-linked-cache-outside");
    std::fs::create_dir_all(&outside).unwrap();
    write(&dir, "recipes/tune.json", SONG);
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    let _ = std::fs::remove_dir_all(dir.join("cache"));
    std::os::unix::fs::symlink(&outside, dir.join("cache")).unwrap();

    let baked = bake_partial(&project, &dir, &id, &Excerpt::default(), None);
    let exported = export_midi(&project, &dir, &id, &[], None);

    assert!(baked.is_err(), "baked through a link");
    assert!(exported.is_err(), "exported through a link");
    let landed: Vec<_> = std::fs::read_dir(&outside).unwrap().collect();
    assert!(landed.is_empty(), "written through the link: {landed:?}");
}
