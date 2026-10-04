//! The shared argument path: the schema it publishes and the refusals it says.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Arguments, ProjectDir, Required, parse, schema};

/// A tool's worth of arguments, of every kind the path has an answer for.
#[derive(Debug, Deserialize, JsonSchema)]
struct Sample {
    project: ProjectDir,
    /// Which clip, as `project_read` names it.
    clip: String,
    /// How many, at most.
    count: Option<u32>,
    /// How far, in seconds.
    seconds: Option<f64>,
    /// Whether to do it.
    #[serde(default)]
    loudly: bool,
    /// Which way.
    way: Option<Way>,
    /// Where, if anywhere.
    spot: Option<Spot>,
}

/// An enum argument, which `schemars` makes nullable when it is optional.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Way {
    Up,
    Down,
}

/// An object argument, which `schemars` wraps in `anyOf` when it is optional.
#[derive(Debug, Deserialize, JsonSchema)]
struct Spot {
    /// Across.
    x: f64,
}

impl Arguments for Sample {
    const REQUIRED: Required = &[("clip", "a clip id")];
}

#[test]
fn the_schema_is_the_shape_the_hand_written_ones_had() {
    assert_eq!(
        schema::<Sample>(),
        json!({
            "type": "object",
            "properties": {
                "project": {
                    "type": "string",
                    "description": "Path to the *.scor project directory to work on."
                },
                "clip": { "type": "string", "description": "Which clip, as `project_read` names it." },
                "count": { "type": "integer", "description": "How many, at most." },
                "seconds": { "type": "number", "description": "How far, in seconds." },
                "loudly": { "type": "boolean", "description": "Whether to do it." },
                "way": { "type": "string", "enum": ["up", "down"], "description": "Which way." },
                "spot": {
                    "type": "object",
                    "description": "Where, if anywhere.",
                    "properties": { "x": { "type": "number", "description": "Across." } },
                    "required": ["x"]
                }
            },
            "required": ["project", "clip"]
        })
    );
}

#[test]
fn a_missing_argument_says_what_it_is() {
    let refused = parse::<Sample>(&json!({ "project": "a.scor" })).unwrap_err();
    assert_eq!(refused, "`clip` is required: a clip id");
    let refused = parse::<Sample>(&json!({ "clip": "c1" })).unwrap_err();
    assert_eq!(
        refused,
        "`project` is required: the path of the *.scor directory"
    );
    let refused = parse::<Sample>(&json!({ "project": "  ", "clip": "c1" })).unwrap_err();
    assert_eq!(
        refused,
        "`project` is required: the path of the *.scor directory"
    );
    let refused = parse::<Sample>(&json!(null)).unwrap_err();
    assert_eq!(
        refused,
        "`project` is required: the path of the *.scor directory"
    );
}

#[test]
fn a_wrong_kind_says_what_it_has_to_be_and_what_it_was() {
    let base = |key: &str, value| {
        let mut arguments = json!({ "project": "a.scor", "clip": "c1" });
        arguments[key] = value;
        parse::<Sample>(&arguments).unwrap_err()
    };
    assert_eq!(
        base("count", json!(2.5)),
        "`count` has to be a whole number, not 2.5"
    );
    assert_eq!(
        base("count", json!(-1)),
        "`count` has to be a whole number, not -1"
    );
    assert_eq!(
        base("seconds", json!("soon")),
        "`seconds` has to be a number, not \"soon\""
    );
    assert_eq!(
        base("loudly", json!("yes")),
        "`loudly` has to be true or false, not \"yes\""
    );
    assert_eq!(base("clip", json!(7)), "`clip` has to be a string, not 7");
    assert_eq!(
        base("way", json!("left")),
        "`way` has to be `up` or `down`, not \"left\""
    );
}

#[test]
fn what_is_left_out_or_unknown_is_read_as_nothing() {
    let read = parse::<Sample>(&json!({
        "project": "a.scor",
        "clip": "c1",
        "count": null,
        "something": "else"
    }))
    .unwrap();
    assert_eq!(read.project.dir(), std::path::Path::new("a.scor"));
    assert_eq!(
        (read.clip.as_str(), read.count, read.seconds),
        ("c1", None, None)
    );
    assert!(!read.loudly);
    assert!(read.way.is_none() && read.spot.is_none());
}

#[test]
fn an_enum_and_an_object_are_read_as_their_types() {
    let read = parse::<Sample>(&json!({
        "project": "a.scor",
        "clip": "c1",
        "way": "down",
        "spot": { "x": 0.25 }
    }))
    .unwrap();
    assert!(matches!(read.way, Some(Way::Down)));
    assert_eq!(read.spot.map(|spot| spot.x), Some(0.25));
}

#[test]
fn a_path_is_the_projects_unless_it_is_absolute() {
    use std::path::{Path, PathBuf};
    let dir = Path::new("/films/a.scor");
    let under = |given| super::under(dir, given, "out");
    assert_eq!(under(None), Ok(None));
    assert_eq!(
        under(Some("x.png")),
        Ok(Some(PathBuf::from("/films/a.scor/x.png")))
    );
    assert_eq!(
        under(Some("/tmp/x.png")),
        Ok(Some(PathBuf::from("/tmp/x.png")))
    );
    assert_eq!(
        super::path(dir, " ", "file"),
        Err("`file` is empty — give a path or leave it out".to_owned())
    );
}

#[test]
fn a_blank_text_is_no_text() {
    assert_eq!(super::given(Some("  pt ")), Some("pt"));
    assert_eq!(super::given(Some("   ")), None);
    assert_eq!(super::given(None), None);
}

/// Schemas nested inside a property, which get the same tidying.
#[derive(Deserialize, JsonSchema)]
struct Nested {
    /// Counts, maybe.
    counts: Option<Vec<u32>>,
    /// Spots, each a count.
    spots: Vec<Count>,
    /// Text or a count.
    either: Option<Either>,
}

/// An object inside a list.
#[derive(Deserialize, JsonSchema)]
struct Count {
    /// How many.
    n: u32,
}

/// An untagged choice, which `schemars` makes nullable with an `anyOf`.
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
enum Either {
    Text(String),
    Count(u32),
}

impl Arguments for Nested {}

#[test]
fn what_is_nested_is_tidied_too() {
    let integer = json!({ "type": "integer" });
    let schema = schema::<Nested>();
    let properties = &schema["properties"];
    assert_eq!(properties["counts"]["items"], integer);
    assert_eq!(properties["counts"]["type"], "array");
    assert_eq!(
        properties["spots"]["items"]["properties"]["n"],
        json!({ "type": "integer", "description": "How many." })
    );
    assert_eq!(
        properties["either"],
        json!({
            "anyOf": [{ "type": "string" }, integer],
            "description": "Text or a count."
        })
    );
}
