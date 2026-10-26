//! The recipe examples in `docs/recipes.md`, held to the parser.
//!
//! That page is what an agent reads before writing a recipe, and an example is
//! the most convincing form a wrong statement can take: it is the part a
//! reader copies rather than interprets. So every marked block there is parsed
//! here, and — because a recipe that parses can still be unplayable — the
//! complete ones are checked by actually rendering them.

mod pages;
#[path = "../../../../zimmer/tests/suite/common/written.rs"]
mod written;

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
                scorsese_zimmer::bake_note(&one_shot.instrument(), midi, &one_shot.opts())
            }),
            // The song example names one instrument by path on purpose, to
            // document that references exist. Resolving it would need a whole
            // project; stubbing it proves the rest of the document, which is
            // what this test is about.
            Recipe::Song(song) => scorsese_zimmer::bake_song(song, &|_: &str| Ok(stub())),
        };
        if let Err(problem) = played {
            panic!(
                "the `{}` example at line {} parses but will not render: {problem}",
                example.marker, example.line
            );
        }
    }
}

/// The page's algorithm table, held to the routings the code publishes.
///
/// An `fm4` recipe picks a routing by name, so a name the page does not list
/// is one nobody can find and a name it lists that the code dropped is one
/// nobody can use. Neither shows up in a parse — the examples only ever name
/// the one or two routings they use — so the table is checked directly.
#[test]
fn the_page_names_every_fm_algorithm() {
    for algorithm in scorsese_zimmer::patch::Algorithm::ALL {
        let row = format!("| `{}` |", algorithm.name());
        assert!(
            pages::DOC.contains(&row),
            "docs/recipes.md has no algorithm row starting `{row}`"
        );
    }
}

/// The page's instrument table, held to the library the way the algorithm
/// table is held to the routings: a name the page lists and the kit dropped is
/// one an agent will write and have refused.
#[test]
fn the_page_names_every_kit_instrument() {
    for instrument in scorsese_providers::synth::kit::KIT {
        let row = format!("| `kit:{}` |", instrument.name);
        assert!(
            pages::DOC.contains(&row),
            "docs/recipes.md has no instrument row starting `{row}`"
        );
    }
}

/// Something audible to stand in for a patch the page names but does not carry.
fn stub() -> scorsese_zimmer::Patch {
    scorsese_zimmer::Patch {
        source: scorsese_zimmer::patch::Source::Noise {
            color: scorsese_zimmer::patch::NoiseColor::White,
        },
        amp: scorsese_zimmer::patch::Adsr::default(),
        filter: None,
        pitch_env: None,
        lfo: None,
        fx: vec![],
    }
}

/// Every worked recipe writes back only the keys it was written with.
///
/// The page's examples reach every stage a recipe has, so a new optional
/// field that writes its default shows up here as a key the example never
/// wrote — the rule `crates/zimmer/CLAUDE.md` states, and
/// `crates/zimmer/tests/suite/common/written.rs` enforces and explains.
#[test]
fn every_example_writes_back_only_what_it_was_written_with() {
    let drifted: Vec<String> = examples()
        .iter()
        .filter_map(|example| {
            let json = document(example);
            let recipe = Recipe::from_json(&json).expect("parsed above");
            let rewritten = serde_json::to_string(&recipe).expect("serialise");
            let what = format!("the `{}` example at line {}", example.marker, example.line);
            written::drift(&what, "", &json, &rewritten)
        })
        .collect();
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}
