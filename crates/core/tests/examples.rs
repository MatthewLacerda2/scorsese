//! The JSON examples in `docs/project-format.md`, held to the parser.
//!
//! That page is what an agent reads before writing a `project.json`, and an
//! example is the most convincing form a wrong statement can take: it is the
//! part a reader copies rather than interprets. So every fenced `json` block
//! there is completed into a whole document here and run through the same
//! loader a real project gets — the convention that makes that possible is
//! documented on the page itself.

use scorsese_core::{Project, SCHEMA_VERSION};
use serde_json::{Value, json};

/// The format contract, embedded at compile time so editing it reruns this.
const DOC: &str = include_str!("../../../docs/project-format.md");

/// One fenced `json` block: what it claims to be, and what is inside it.
struct Example {
    marker: String,
    body: String,
    line: usize,
}

/// Every fenced `json` block on the page, in order.
fn examples() -> Vec<Example> {
    let mut found = Vec::new();
    let mut lines = DOC.lines().enumerate();
    while let Some((number, line)) = lines.next() {
        let Some(marker) = line.strip_prefix("```json") else {
            continue;
        };
        let body: Vec<&str> = lines
            .by_ref()
            .map(|(_, line)| line)
            .take_while(|line| !line.starts_with("```"))
            .collect();
        found.push(Example {
            marker: marker.trim().to_owned(),
            body: body.join("\n"),
            line: number + 1,
        });
    }
    found
}

/// The whole document this block is a piece of, ready to be loaded.
///
/// A fragment is put in the smallest project that could contain it. Nothing
/// invents assets to satisfy the references a fragment makes, which is why
/// only `project` blocks are validated below.
fn document(example: &Example) -> String {
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

    let mut base = json!({
        "schema_version": SCHEMA_VERSION,
        "name": "example",
        "timeline_fps": { "num": 30, "den": 1 },
    });
    let object = base.as_object_mut().expect("the base is an object");
    match example.marker.as_str() {
        "project" => return fragment.to_string(),
        "fields" => {
            let fields = fragment.as_object().expect("`fields` wraps into one");
            for (key, value) in fields {
                object.insert(key.clone(), value.clone());
            }
        }
        "asset" => {
            object.insert("assets".to_owned(), json!([fragment]));
        }
        "track" => {
            object.insert("tracks".to_owned(), json!([fragment]));
        }
        "clip" => {
            let track = json!({ "id": "t", "kind": "video", "clips": [fragment] });
            object.insert("tracks".to_owned(), json!([track]));
        }
        other => panic!(
            "example at line {}: `{other}` is not a marker this test knows. \
             Mark the block `project`, `fields`, `asset`, `track` or `clip`.",
            example.line
        ),
    }
    base.to_string()
}

#[test]
fn the_page_still_carries_examples() {
    let examples = examples();
    assert!(
        examples.len() >= 6,
        "found only {} json blocks — the extractor is probably broken, \
         not the page",
        examples.len()
    );
    assert!(
        examples.iter().any(|example| example.marker == "project"),
        "no complete `project` example is left on the page"
    );
}

#[test]
fn every_example_parses() {
    for example in examples() {
        let json = document(&example);
        if let Err(error) = Project::from_json(&json) {
            panic!(
                "`{}` example at line {} does not load: {error}\ncompleted to: {json}",
                example.marker, example.line
            );
        }
    }
}

#[test]
fn the_complete_documents_also_validate() {
    for example in examples().iter().filter(|it| it.marker == "project") {
        let project = Project::from_json(&document(example)).expect("parsed above");
        assert_eq!(
            project.validate(),
            Ok(()),
            "the `project` example at line {} is invalid",
            example.line
        );
    }
}
