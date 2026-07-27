//! The recipe examples in `docs/recipes.md`, held to the parser.
//!
//! That page is what an agent reads before writing a recipe, and an example is
//! the most convincing form a wrong statement can take: it is the part a
//! reader copies rather than interprets. So every fenced block marked `recipe`
//! there is parsed here, and — because a recipe that parses can still be
//! unplayable — validated by actually rendering it.

use scorsese_providers::Recipe;

/// The page, embedded at compile time so editing it reruns this.
const DOC: &str = include_str!("../../../docs/recipes.md");

/// Every ` ```json recipe ` block on the page, with the line it starts on.
///
/// The marker is required: a bare ` ```json ` block would quietly escape the
/// check, so a new example has to say what it is. Illustrative fragments are
/// marked `jsonc` and are not claimed to be complete.
fn examples() -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut lines = DOC.lines().enumerate();
    while let Some((number, line)) = lines.next() {
        if line.trim_end() != "```json recipe" {
            assert!(
                line.trim_end() != "```json",
                "line {}: an unmarked ```json block would escape this check — \
                 mark it ```json recipe, or ```jsonc if it is a fragment",
                number + 1
            );
            continue;
        }
        let body: Vec<&str> = lines
            .by_ref()
            .map(|(_, line)| line)
            .take_while(|line| !line.starts_with("```"))
            .collect();
        found.push((number + 1, body.join("\n")));
    }
    found
}

#[test]
fn the_page_still_carries_examples() {
    let examples = examples();
    assert!(
        examples.len() >= 2,
        "found {} — one of each shape is the minimum the page documents",
        examples.len()
    );
}

#[test]
fn every_example_parses() {
    for (line, body) in examples() {
        if let Err(problem) = Recipe::from_json(&body) {
            panic!("the recipe at line {line} does not parse: {problem}\n{body}");
        }
    }
}

/// Parsing is not enough. A patch whose stack is empty parses perfectly and
/// makes no sound, and an example nobody can render is worse than no example.
#[test]
fn every_example_is_actually_playable() {
    for (line, body) in examples() {
        let recipe = Recipe::from_json(&body).expect("parsed above");
        let played = match &recipe {
            Recipe::Patch(one_shot) => one_shot.note.to_midi().and_then(|midi| {
                scorsese_soundgen::bake_note(&one_shot.patch, midi, &one_shot.opts())
            }),
            // The song example names one instrument by path on purpose, to
            // document that references exist. Resolving it here would need a
            // project; rendering it with the reference stubbed out proves the
            // rest of the document — which is what this test is about.
            Recipe::Song(song) => {
                scorsese_soundgen::bake_song(song, &|_: &str| Ok(stub_instrument()))
            }
        };
        if let Err(problem) = played {
            panic!("the recipe at line {line} parses but will not render: {problem}");
        }
    }
}

/// Something audible to stand in for a patch the page names but does not carry.
fn stub_instrument() -> scorsese_soundgen::Patch {
    scorsese_soundgen::Patch {
        source: scorsese_soundgen::patch::Source::Noise,
        amp: scorsese_soundgen::patch::Adsr::default(),
        filter: None,
        lfo: None,
        fx: vec![],
    }
}
