//! The shared argument path: the schema it publishes and the refusals it says.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Arguments, ProjectDir, parse, schema};

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
}

impl Arguments for Sample {
    const REQUIRED: &'static [(&'static str, &'static str)] = &[("clip", "a clip id")];
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
                "loudly": { "type": "boolean", "description": "Whether to do it." }
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
}

#[test]
fn a_blank_text_is_no_text() {
    assert_eq!(super::given(Some("  pt ")), Some("pt"));
    assert_eq!(super::given(Some("   ")), None);
    assert_eq!(super::given(None), None);
}
