//! Every tool, and every argument of every tool, says what it is.
//!
//! This is the gate the crate doc promises. A description is the entire
//! interface a client has to a tool: an undescribed one is a capability that
//! exists and cannot be found, and nothing fails — the assistant on the other
//! end simply never calls it. That failure is invisible, which is exactly why
//! it needs a test rather than a review.
//!
//! The same gate already covers the CLI's `--help` and the animatable-property
//! table in `docs/project-format.md`. Walking a tool registry is that test
//! again.
//!
//! **What a tool costs is held to the same discipline, and not from here.**
//! `Tool::costs` is a required method with no default, so a tool that does not
//! state what calling it spends fails to compile — which is a stronger gate
//! than any assertion this file could make, and the reason there is no test for
//! it below. A test that merely re-checked what the type system already refuses
//! would read as coverage while proving nothing.
//!
//! The half that *is* testable moved to `tests/table.rs`: a description's first
//! sentence is the cell `docs/mcp.md` carries, so it has to stand on its own.

use scorsese_mcp::registry;
use serde_json::Value;

/// Long enough to be a sentence rather than a restatement of the name.
const MIN_DESCRIPTION: usize = 40;

#[test]
fn the_registry_is_not_empty() {
    assert!(
        !registry().is_empty(),
        "no tools — every test below would pass vacuously"
    );
}

#[test]
fn every_tool_says_what_it_does() {
    for tool in registry() {
        let description = tool.description();
        assert!(
            description.len() >= MIN_DESCRIPTION,
            "`{}` describes itself in {} characters, which is not a description",
            tool.name(),
            description.len()
        );
        assert!(
            !description.trim().eq_ignore_ascii_case(tool.name()),
            "`{}`'s description only repeats its name",
            tool.name()
        );
    }
}

#[test]
fn every_tool_name_is_usable_and_unique() {
    let mut seen = Vec::new();
    for tool in registry() {
        let name = tool.name();
        assert!(
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "`{name}` is not a usable tool name: lowercase, digits and underscores"
        );
        assert!(!seen.contains(&name), "two tools are both called `{name}`");
        seen.push(name);
    }
}

/// A schema is what a client fills in. A property nobody described is a field
/// an assistant has to guess at, and guessing at `depth` is how a mix ends up
/// silent.
#[test]
fn every_argument_of_every_tool_says_what_it_is() {
    for tool in registry() {
        let schema = tool.schema();
        assert_eq!(
            schema.get("type").and_then(Value::as_str),
            Some("object"),
            "`{}`'s schema is not an object",
            tool.name()
        );
        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("`{}`'s schema has no properties", tool.name()));

        for (argument, described) in properties {
            let description = described.get("description").and_then(Value::as_str);
            assert!(
                description.is_some_and(|text| text.len() >= 12),
                "`{}`'s `{argument}` argument says nothing useful about itself",
                tool.name()
            );
            assert!(
                described.get("type").is_some(),
                "`{}`'s `{argument}` argument has no type",
                tool.name()
            );
        }
    }
}

/// Every tool works on a project, and every one of them must ask for it the
/// same way — a client should never have to remember that one tool spells it
/// `path` and another `dir`.
#[test]
fn every_tool_takes_the_project_the_same_way() {
    for tool in registry() {
        let schema = tool.schema();
        let required: Vec<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        assert!(
            required.contains(&"project"),
            "`{}` does not require a `project`",
            tool.name()
        );
    }
}
