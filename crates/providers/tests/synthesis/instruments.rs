//! A song whose instrument lives in its own file — the reference a
//! `PatchResolver` exists to resolve, wired to a real project.

use crate::common::{project, synth_asset, write};
use scorsese_providers::synth::{SynthesisError, bake_asset};

const BASS: &str = r#"{
  "source": { "kind": "karplus", "damping": 0.99, "brightness": 0.4 },
  "amp": { "a": 0.001, "d": 0.2, "s": 0.0, "r": 0.1 }
}"#;

/// A song naming its instrument at `reference` rather than carrying it inline.
fn song_naming(reference: &str) -> String {
    format!(
        r#"{{
  "recipe": "song", "bpm": 120, "seed": 3,
  "tracks": [{{ "name": "bass", "patch": "{reference}", "gain": 0.9 }}],
  "patterns": {{ "a": {{ "beats": 2, "notes": [
      {{ "track": "bass", "note": "E2", "start": 0, "dur": 0.5 }} ] }} }},
  "arrangement": ["a"]
}}"#
    )
}

#[test]
fn a_named_instrument_is_read_from_the_project() {
    let (dir, mut project) = project("named-instrument");
    write(&dir, "recipes/bass.json", BASS);
    write(&dir, "recipes/tune.json", &song_naming("recipes/bass.json"));

    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    let baked = bake_asset(&mut project, &dir, &id).expect("bakes");
    assert!(baked.is_fresh());

    let wav = std::fs::read(baked.path().resolve(&dir)).expect("the baked file");
    assert!(
        wav[44..].iter().any(|&byte| byte != 0),
        "a resolved instrument must actually play"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The resolver runs the same path rules the recipe itself does, so an
/// instrument reference is not a second, looser way into the filesystem.
#[test]
fn a_named_instrument_may_not_climb_out_of_the_project() {
    let (dir, mut project) = project("instrument-escape");
    write(&dir, "recipes/tune.json", &song_naming("../bass.json"));
    std::fs::write(dir.parent().expect("a parent").join("bass.json"), BASS)
        .expect("write it outside");

    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    let message = refused.to_string();
    assert!(
        matches!(refused, SynthesisError::Unrenderable { .. }),
        "got {message}"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_instrument_that_is_not_there_names_its_track_and_its_reference() {
    let (dir, mut project) = project("instrument-missing");
    write(&dir, "recipes/tune.json", &song_naming("recipes/nope.json"));

    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    let refused = bake_asset(&mut project, &dir, &id).expect_err("refused");
    let message = refused.to_string();
    let source = std::error::Error::source(&refused)
        .expect("the synth error is the source")
        .to_string();
    assert!(source.contains("bass"), "names the track: {source}");
    assert!(
        source.contains("recipes/nope.json"),
        "and the reference: {source}"
    );
    assert!(message.contains("tune.json"), "and the recipe: {message}");
    std::fs::remove_dir_all(dir).ok();
}

/// Two songs sharing an instrument is exactly why references exist, so the
/// shared file must be read for each of them rather than the first winning.
#[test]
fn two_songs_may_share_one_instrument_file() {
    let (dir, mut project) = project("instrument-shared");
    write(&dir, "recipes/bass.json", BASS);
    write(&dir, "recipes/one.json", &song_naming("recipes/bass.json"));
    write(
        &dir,
        "recipes/two.json",
        &song_naming("recipes/bass.json").replace(r#""bpm": 120"#, r#""bpm": 90"#),
    );

    let one = synth_asset(&mut project, "one", "recipes/one.json");
    let two = synth_asset(&mut project, "two", "recipes/two.json");
    let first = bake_asset(&mut project, &dir, &one).expect("bakes");
    let second = bake_asset(&mut project, &dir, &two).expect("bakes");
    assert_ne!(
        first.path(),
        second.path(),
        "different tempos are different bakes"
    );
    std::fs::remove_dir_all(dir).ok();
}
