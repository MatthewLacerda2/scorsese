//! The recipe examples in `docs/recipes.md`, held to the parser.
//!
//! That page is what an agent reads before writing a recipe, and an example is
//! the most convincing form a wrong statement can take: it is the part a
//! reader copies rather than interprets. So every marked block there is parsed
//! here, and — because a recipe that parses can still be unplayable — the
//! complete ones are checked by actually rendering them.

mod pages;

use pages::{document, examples};
use scorsese_providers::synth::Recipe;

#[test]
fn the_page_still_carries_examples() {
    let markers: Vec<String> = examples().into_iter().map(|it| it.marker).collect();
    assert!(
        markers.iter().filter(|it| *it == "recipe").count() >= 2,
        "one complete example of each shape is the minimum, got {markers:?}"
    );
}

#[test]
fn every_example_parses() {
    for example in examples() {
        let json = document(&example);
        if let Err(problem) = Recipe::from_json(&json) {
            panic!(
                "the `{}` example at line {} does not parse: {problem}\n{json}",
                example.marker, example.line
            );
        }
    }
}

/// Parsing is not enough. A patch whose stack is empty parses perfectly and
/// makes no sound, and an example nobody can play is worse than no example.
#[test]
fn every_example_is_actually_playable() {
    for example in examples() {
        let recipe = Recipe::from_json(&document(&example)).expect("parsed above");
        let played = match &recipe {
            Recipe::Patch(one_shot) => one_shot.note.to_midi().and_then(|midi| {
                scorsese_soundgen::bake_note(&one_shot.patch, midi, &one_shot.opts())
            }),
            // The song example names one instrument by path on purpose, to
            // document that references exist. Resolving it would need a whole
            // project; stubbing it proves the rest of the document, which is
            // what this test is about.
            Recipe::Song(song) => scorsese_soundgen::bake_song(song, &|_: &str| Ok(stub())),
        };
        if let Err(problem) = played {
            panic!(
                "the `{}` example at line {} parses but will not render: {problem}",
                example.marker, example.line
            );
        }
    }
}

/// Something audible to stand in for a patch the page names but does not carry.
fn stub() -> scorsese_soundgen::Patch {
    scorsese_soundgen::Patch {
        source: scorsese_soundgen::patch::Source::Noise,
        amp: scorsese_soundgen::patch::Adsr::default(),
        filter: None,
        lfo: None,
        fx: vec![],
    }
}
