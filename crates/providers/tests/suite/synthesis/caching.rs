//! What a second bake of an unchanged recipe says, what a bake writes onto
//! its asset, and the reads that cost nothing: a partial bake's refusal and a
//! survey of the set.

use crate::common::{project, synth_asset, write};
use scorsese_core::AssetKind;
use scorsese_providers::synth::{Baked, Excerpt, SynthesisError, bake_asset, bake_partial, survey};
use scorsese_zimmer::SAMPLE_RATE;

/// Two sections, so a cached bake has sections to say.
const TUNE: &str = r#"{
  "recipe": "song", "bpm": 120, "seed": 3,
  "tracks": [{ "name": "bass", "patch": {
      "source": { "kind": "noise" },
      "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } }],
  "patterns": {
    "verse": { "beats": 2, "notes": [
      { "track": "bass", "note": "E2", "start": 0, "dur": 0.5 } ] },
    "chorus": { "beats": 2, "notes": [
      { "track": "bass", "note": "B2", "start": 0, "dur": 0.5 } ] } },
  "arrangement": ["verse", "chorus"]
}"#;

#[test]
fn a_second_bake_is_a_cache_hit_that_still_says_its_sections() {
    let (dir, mut project) = project("cache-hit");
    write(&dir, "recipes/tune.json", TUNE);
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");

    let first = bake_asset(&mut project, &dir, &id).expect("bakes");
    assert!(first.is_fresh(), "the first bake renders");
    let again = bake_asset(&mut project, &dir, &id).expect("bakes");
    assert!(!again.is_fresh(), "the second is served from disk");
    assert_eq!(again.path(), first.path());
    let Baked::Cached { sections, .. } = &again else {
        panic!("a cache hit: {again:?}")
    };
    let labels: Vec<&str> = sections.iter().map(|cut| cut.label.as_str()).collect();
    assert_eq!(labels, ["verse", "chorus"]);
    std::fs::remove_dir_all(dir).ok();
}

/// The asset says what is in the file, so nobody has to probe a bake.
#[test]
fn a_bake_writes_its_shape_onto_the_asset() {
    let (dir, mut project) = project("bake-shape");
    write(&dir, "recipes/tune.json", TUNE);
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    bake_asset(&mut project, &dir, &id).expect("bakes");

    let media = project.asset(&id).and_then(|asset| asset.media);
    let media = media.expect("the bake recorded its metadata");
    assert_eq!(media.audio_channels, Some(2));
    assert_eq!(media.sample_rate, Some(SAMPLE_RATE));
    assert!(media.duration_seconds.is_some_and(|seconds| seconds > 1.0));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_partial_bake_refuses_an_asset_that_is_not_a_recipe() {
    let (dir, mut project) = project("partial-refused");
    write(&dir, "recipes/tune.json", TUNE);
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    project.assets[0].kind = AssetKind::Audio;
    let refused =
        bake_partial(&project, &dir, &id, &Excerpt::default(), None).expect_err("not synthesised");
    assert!(
        matches!(refused, SynthesisError::NotSynthesised { .. }),
        "{refused}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A partial is named for its asset, and an id is whatever the document says:
/// one that climbs never names a file outside the project.
#[test]
fn a_partial_bake_of_an_id_that_climbs_writes_nothing_outside() {
    let (dir, mut project) = project("partial-climbs");
    write(&dir, "recipes/tune.json", TUNE);
    let name = format!(
        "{}-escaped",
        dir.file_stem()
            .and_then(|stem| stem.to_str())
            .expect("a name")
    );
    let id = synth_asset(
        &mut project,
        &format!("../../../{name}"),
        "recipes/tune.json",
    );
    let outside = dir.parent().expect("a parent").join(format!("{name}.wav"));

    let outcome = bake_partial(&project, &dir, &id, &Excerpt::default(), None);
    let escaped = outside.exists();
    std::fs::remove_file(&outside).ok();
    std::fs::remove_dir_all(dir).ok();
    assert!(!escaped, "a file was written outside the project");
    assert!(outcome.is_err(), "{outcome:?}");
}

#[test]
fn a_survey_reads_every_song_by_its_asset() {
    let (dir, mut project) = project("survey");
    write(&dir, "recipes/tune.json", TUNE);
    synth_asset(&mut project, "tune", "recipes/tune.json");
    let found = survey(&project, &dir).expect("surveys");
    let names: Vec<&str> = found.songs.iter().map(|song| song.name.as_str()).collect();
    assert_eq!(names, ["tune"]);
    assert_eq!(found.songs[0].bpm, 120.0);
    std::fs::remove_dir_all(dir).ok();
}
