use super::*;
use scorsese_zimmer::{NoteOpts, bake_named_note};

mod effects;

pub(super) fn bands_of(instrument: &Instrument) -> (u32, u32, u32) {
    let opts = NoteOpts {
        duration: 1.0,
        ..NoteOpts::default()
    };
    let bake = bake_named_note(&instrument.patch(), instrument.home, &opts)
        .unwrap_or_else(|error| panic!("{} does not render: {error}", instrument.name));
    assert!(
        bake.loudness().mean_dbfs > Some(-40.0),
        "{} is near-silent: {:?}",
        instrument.name,
        bake.loudness()
    );
    bake.profile
        .whole
        .bands
        .as_ref()
        .expect("a one-second bake is long enough to split")
        .percentages()
}

#[test]
fn every_instrument_renders_at_its_home_note() {
    for instrument in KIT {
        bands_of(instrument);
    }
}

/// The library's promise is that a name sounds like what it names, and where
/// the energy sits is the part of that a number can hold: a kick that is not
/// mostly low, or a hat that is not mostly high, is the wrong instrument
/// whatever its label says.
#[test]
fn each_instrument_sits_where_its_name_says() {
    let low = |name: &str| bands_of(lookup(name).expect(name)).0;
    let high = |name: &str| bands_of(lookup(name).expect(name)).2;
    assert!(low("kick") > 70, "kick low share {}", low("kick"));
    assert!(low("bass") > 50, "bass low share {}", low("bass"));
    assert!(high("hat") > 80, "hat high share {}", high("hat"));
    assert!(high("crash") > 60, "crash high share {}", high("crash"));
    let (snare_low, snare_mid, _) = bands_of(lookup("snare").expect("snare"));
    assert!(snare_mid > snare_low, "a snare is a crack, not a thud");
}

#[test]
fn names_are_unique_and_found_with_or_without_the_prefix() {
    for (index, instrument) in KIT.iter().enumerate() {
        assert!(
            KIT[index + 1..].iter().all(|it| it.name != instrument.name),
            "{} twice",
            instrument.name
        );
        assert_eq!(lookup(instrument.name), Some(instrument));
        assert_eq!(
            lookup(&format!("kit:{}", instrument.name)),
            Some(instrument)
        );
    }
    assert_eq!(lookup("kazoo"), None);
}

const SONG: &str = r#"{ "recipe": "song", "bpm": 120,
  "tracks": [
    { "name": "kick", "patch": "kit:kick" },
    { "name": "lead", "patch": "recipes/lead.json" }
  ],
  "patterns": { "a": { "beats": 4, "notes": [
    { "track": "kick", "steps": "x---x---", "div": 0.5 } ] } },
  "arrangement": ["a"] }"#;

#[test]
fn a_kit_name_in_a_song_becomes_the_patch_itself() {
    let expanded = expand(SONG).expect("expands");
    assert_eq!(expanded.copied, ["kick"]);
    let Recipe::Song(song) = &expanded.recipe else {
        panic!("still a song")
    };
    let kick = lookup("kick").expect("kick").patch();
    assert_eq!(
        song.tracks[0].patch,
        scorsese_zimmer::song::PatchRef::Inline(Box::new(kick))
    );
    // A path is the author's own reference, and stays one.
    assert_eq!(
        song.tracks[1].patch,
        scorsese_zimmer::song::PatchRef::Named("recipes/lead.json".to_owned())
    );
    assert!(!expanded.json.contains("kit:"), "{}", expanded.json);
    assert_eq!(
        Recipe::from_json(&expanded.json).expect("parses"),
        expanded.recipe
    );
}

#[test]
fn a_one_shot_can_name_a_kit_instrument() {
    let json = r#"{ "recipe": "patch", "note": "C4", "patch": "kit:snare" }"#;
    let expanded = expand(json).expect("expands");
    assert_eq!(expanded.copied, ["snare"]);
    let Recipe::Patch(one_shot) = expanded.recipe else {
        panic!("still a one-shot")
    };
    assert_eq!(one_shot.patch, lookup("snare").expect("snare").patch());
}

#[test]
fn a_recipe_with_no_kit_name_is_left_byte_for_byte() {
    let json = SONG.replace("kit:kick", "recipes/kick.json");
    let expanded = expand(&json).expect("parses");
    assert!(expanded.copied.is_empty());
    assert_eq!(expanded.json, json);
}

#[test]
fn an_unknown_name_is_refused_with_the_list() {
    let problem = expand(&SONG.replace("kit:kick", "kit:cowbell"))
        .expect_err("no cowbell")
        .to_string();
    assert!(problem.contains("kit:cowbell"), "{problem}");
    assert!(problem.contains("kick, snare"), "{problem}");
}

/// What `synth_kit` shows is what `kit:` copies in, not a description of it.
#[test]
fn the_json_shown_is_the_patch_copied() {
    for instrument in KIT {
        let shown = Patch::from_json(instrument.json()).expect("the shown JSON parses");
        assert_eq!(shown, instrument.patch(), "{}", instrument.name);
    }
}
