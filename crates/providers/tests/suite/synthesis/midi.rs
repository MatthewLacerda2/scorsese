//! MIDI both ways, through a real project: a song exported to a `.mid`, and
//! that file read back in as a new song. The conversions are `zimmer`'s and
//! tested there; what is asserted here is the half that touches a disk, and
//! the report both front doors print.

use crate::common::{project, synth_asset, write};
use scorsese_core::{AssetKind, ProjectPath};
use scorsese_providers::synth::{
    Drum, FromMidi, MIDI_EXPORT_DIR, SynthesisError, ToMidi, export_midi, import_midi,
};

/// Two tracks, three notes, one tempo.
const SONG: &str = r#"{
  "recipe": "song", "bpm": 100, "seed": 1,
  "tracks": [
    { "name": "lead", "patch": { "source": { "kind": "noise" },
        "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } },
    { "name": "kick", "patch": { "source": { "kind": "noise" },
        "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } }
  ],
  "patterns": { "a": { "beats": 4, "notes": [
      { "track": "lead", "note": "C4", "start": 0, "dur": 1 },
      { "track": "lead", "note": "E4", "start": 1, "dur": 1 },
      { "track": "kick", "note": "C2", "start": 0, "dur": 0.5 } ] } },
  "arrangement": ["a"]
}"#;

const ONE_SHOT: &str = r#"{ "recipe": "patch", "note": "C4",
  "patch": { "source": { "kind": "noise" },
             "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } }"#;

#[test]
fn a_song_goes_out_to_the_cache_and_comes_back_as_a_new_song() {
    let (dir, mut project) = project("midi-round-trip");
    write(&dir, "recipes/tune.json", SONG);
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");

    let out = export_midi(&project, &dir, &id, &[], None).expect("exports");
    let expected = format!("{MIDI_EXPORT_DIR}/tune.mid");
    assert_eq!(out.shown, expected, "said project-relative");
    assert_eq!(out.file, ProjectPath::new(&expected).resolve(&dir));
    let bytes = std::fs::read(&out.file).expect("the file is on disk");
    assert_eq!(out.bytes, bytes.len());
    assert_eq!((out.tracks, out.notes), (2, 3));
    assert!(out.drums.is_empty());

    let back = import_midi(&mut project, &dir, &out.file, None).expect("imports");
    let named = back.id.as_str();
    assert!(
        named.starts_with("tune") && named != "tune",
        "named after the file, out of the way: {named}"
    );
    assert_eq!((back.tracks, back.notes), (2, 3), "every note came back");
    assert_eq!(back.bpm, 100.0);
    let asset = project.asset(&back.id).expect("the import added an asset");
    assert_eq!(asset.kind, AssetKind::SynthAudio);
    let recipe = asset.recipe.as_ref().expect("it points at its recipe");
    assert!(recipe.resolve(&dir).is_file(), "and the recipe was written");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_export_goes_where_it_is_told_with_its_drums_named() {
    let (dir, mut project) = project("midi-out");
    write(&dir, "recipes/tune.json", SONG);
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    let chosen = dir.join("elsewhere.mid");
    let kick: Drum = "kick=36".parse().expect("a drum");

    let out = export_midi(
        &project,
        &dir,
        &id,
        std::slice::from_ref(&kick),
        Some(&chosen),
    )
    .expect("exports");
    assert_eq!(out.file, chosen);
    assert_eq!(
        out.shown,
        chosen.display().to_string(),
        "as the caller wrote it"
    );
    assert_eq!(out.drums, vec![kick]);
    assert!(chosen.is_file());
    assert!(
        out.lines()[0].ends_with(", drums on channel 10: kick=36"),
        "{:?}",
        out.lines()
    );

    let named = import_midi(&mut project, &dir, &chosen, Some("again")).expect("imports");
    assert_eq!(named.id.as_str(), "again", "a name given is the name used");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn only_a_synthesised_song_exports() {
    let (dir, mut project) = project("midi-refused");
    write(&dir, "recipes/pop.json", ONE_SHOT);
    let pop = synth_asset(&mut project, "pop", "recipes/pop.json");
    let refused = export_midi(&project, &dir, &pop, &[], None).expect_err("a one-shot");
    assert!(
        matches!(refused, SynthesisError::NoScore { .. }),
        "{refused}"
    );

    project.assets[0].kind = AssetKind::Audio;
    let refused = export_midi(&project, &dir, &pop, &[], None).expect_err("not a recipe");
    assert!(
        matches!(refused, SynthesisError::NotSynthesised { .. }),
        "{refused}"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_export_reports_its_shape_and_what_it_left_behind() {
    let out = ToMidi {
        file: "x.mid".into(),
        shown: "x.mid".into(),
        bytes: 120,
        tracks: 2,
        notes: 9,
        tempo_events: 1,
        drums: Vec::new(),
        left_out: vec!["swing".into()],
    };
    let lines = out.lines();
    assert_eq!(lines[0], "2 track(s), 9 notes, 1 tempo event(s), 120 bytes");
    assert_eq!(lines[1], "left out: swing");
    assert!(lines[2].starts_with("the file is the notes and the tempo"));
    assert_eq!(lines.len(), 3);
}

#[test]
fn an_import_reports_tempo_changes_and_key_only_when_there_are_some() {
    let mut from = FromMidi {
        id: scorsese_core::AssetId::new("tune"),
        tracks: 1,
        notes: 4,
        patterns: 1,
        bpm: 120.0,
        tempo_changes: 0,
        key: None,
        left_out: Vec::new(),
    };
    let plain = from.lines();
    assert!(plain[0].ends_with("at 120 bpm"), "{plain:?}");
    assert!(plain[1].starts_with("every track plays a plain default patch"));
    assert_eq!(plain.len(), 2);

    from.tempo_changes = 1;
    from.key = Some("A minor".into());
    from.left_out = vec!["pitch bends".into()];
    let full = from.lines();
    assert!(
        full[0].ends_with("at 120 bpm with 1 tempo change(s), in A minor"),
        "{full:?}"
    );
    assert_eq!(full[1], "left out: pitch bends");
    assert_eq!(full.len(), 3);
}
