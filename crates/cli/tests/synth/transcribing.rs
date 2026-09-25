//! `synth import`: a MIDI file in as a song recipe.

use crate::common::{holds, new_project, reload, run_in};
use scorsese_core::AssetKind;

/// The smallest useful file: format 0, 96 ticks a beat, middle C for one beat
/// and then E for one, on channel 1, with a sustain-pedal press the song has
/// no room for. Written out byte by byte so the fixture cannot agree with the
/// parser about a mistake.
const TUNE: &[u8] = &[
    b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, //
    b'M', b'T', b'r', b'k', 0, 0, 0, 24, //
    0x00, 0xB0, 64, 127, // pedal down
    0x00, 0x90, 60, 100, // C4 on
    0x60, 0x80, 60, 0, // C4 off, a beat later
    0x00, 0x90, 64, 100, // E4 on
    0x60, 0x80, 64, 0, // E4 off
    0x00, 0xFF, 0x2F, 0x00, // end of track
];

#[test]
fn import_writes_a_song_recipe_named_for_the_file_and_it_bakes() {
    let dir = new_project("synth-import");
    let file = dir.join("tune.mid");
    std::fs::write(&file, TUNE).expect("write the fixture");

    let said = run_in(&dir, &["synth", "import", file.to_str().expect("utf-8")]).ok();
    assert!(
        said.output.contains("tune — synth_audio, sketch"),
        "{}",
        said.output
    );
    assert!(
        said.output.contains("1 track(s), 2 notes"),
        "{}",
        said.output
    );
    assert!(
        said.output.contains("left out: 1 controller changes"),
        "the pedal is named, not dropped: {}",
        said.output
    );

    assert!(holds(&dir, "recipes/tune.json"), "the recipe is written");
    let recipe = std::fs::read_to_string(dir.join("recipes/tune.json")).expect("the recipe");
    assert!(recipe.contains("\"recipe\": \"song\""), "{recipe}");
    assert!(recipe.contains("\"E4\""), "{recipe}");
    let project = reload(&dir);
    let asset = project.assets.first().expect("one asset");
    assert_eq!(asset.kind, AssetKind::SynthAudio);
    assert!(asset.needs_generation(), "it starts as a sketch");

    run_in(&dir, &["synth", "bake", "tune"]).ok();
    assert!(
        reload(&dir).assets[0].path.is_some(),
        "and bakes as written"
    );
}

#[test]
fn a_name_can_be_given_and_a_file_that_is_not_midi_is_refused() {
    let dir = new_project("synth-import-named");
    let file = dir.join("tune.mid");
    std::fs::write(&file, TUNE).expect("write the fixture");
    let path = file.to_str().expect("utf-8");
    run_in(&dir, &["synth", "import", path, "--name", "rag"]).ok();
    assert!(holds(&dir, "recipes/rag.json"));

    std::fs::write(&file, b"not midi at all").expect("overwrite");
    let refused = run_in(&dir, &["synth", "import", path]);
    assert!(refused.failed, "{}", refused.output);
    assert!(
        refused.output.contains("not a readable MIDI file"),
        "{}",
        refused.output
    );
    assert_eq!(reload(&dir).assets.len(), 1, "nothing was added");
}
