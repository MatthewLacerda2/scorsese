//! Pulling the examples out of a Markdown page and completing them.
//!
//! The marker convention is documented on the page itself, so a reader of
//! either can check the other.

use serde_json::{Value, json};

/// The page, embedded at compile time so editing it reruns this.
pub(crate) const DOC: &str = include_str!("../../../../docs/recipes.md");

/// One fenced block: what it claims to be, and what is inside it.
pub(crate) struct Example {
    pub(crate) marker: String,
    pub(crate) body: String,
    pub(crate) line: usize,
}

/// Every marked ` ```json ` block on the page.
///
/// The marker is required — a bare ` ```json ` block would quietly escape this
/// check, so a new example has to say what it is. `recipe` is a whole recipe;
/// `fields` is top-level fields spliced into a minimal song. Illustrative
/// fragments that are neither are marked `jsonc` and are not fenced as JSON at
/// all.
pub(crate) fn examples() -> Vec<Example> {
    let mut found = Vec::new();
    let mut lines = DOC.lines().enumerate();
    while let Some((number, line)) = lines.next() {
        let Some(marker) = line.trim_end().strip_prefix("```json") else {
            continue;
        };
        let marker = marker.trim().to_owned();
        assert!(
            !marker.is_empty(),
            "line {}: an unmarked ```json block would escape this check — \
             mark it `recipe` or `fields`",
            number + 1
        );
        let body: Vec<&str> = lines
            .by_ref()
            .map(|(_, line)| line)
            .take_while(|line| !line.starts_with("```"))
            .collect();
        found.push(Example {
            marker,
            body: body.join("\n"),
            line: number + 1,
        });
    }
    found
}

/// The whole recipe this block is a piece of.
///
/// A `fields` block is spliced into the smallest song that could carry it, so
/// documenting a field proves the field exists rather than proving the page
/// compiles.
pub(crate) fn document(example: &Example) -> String {
    let fragment: Value = match example.marker.as_str() {
        "fields" => serde_json::from_str(&format!("{{{}}}", example.body)),
        _ => serde_json::from_str(&example.body),
    }
    .unwrap_or_else(|error| {
        panic!(
            "`{}` example at line {}: not JSON at all: {error}",
            example.marker, example.line
        )
    });

    match example.marker.as_str() {
        "recipe" => fragment.to_string(),
        "fields" => {
            let mut song = minimal_song();
            let object = song.as_object_mut().expect("the base is an object");
            for (key, value) in fragment.as_object().expect("`fields` wraps into one") {
                object.insert(key.clone(), value.clone());
            }
            song.to_string()
        }
        other => panic!(
            "example at line {}: `{other}` is not a marker this test knows",
            example.line
        ),
    }
}

/// The smallest song a `fields` block can be spliced into.
fn minimal_song() -> Value {
    json!({
        "recipe": "song",
        "bpm": 120,
        "tracks": [{ "name": "t", "patch": {
            "source": { "kind": "noise" },
            "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 }
        } }],
        "patterns": { "a": { "beats": 4, "notes": [
            { "track": "t", "note": "C3", "start": 0, "dur": 1.0 }
        ] } },
        "arrangement": ["a"]
    })
}
