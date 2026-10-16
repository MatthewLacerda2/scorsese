//! Every tool refuses an argument it does not take (#1117), and a key inside
//! an object argument whose schema lists the keys it takes (#1330).
//!
//! The rule lives in one place — the shared argument parser — and this holds
//! every tool to going through it: a tool that read its arguments some other
//! way would quietly skip a misspelt one again, and run without it.

use scorsese_mcp::registry;
use serde_json::json;

#[test]
fn every_tool_refuses_an_argument_it_does_not_take() {
    let project = std::env::temp_dir().join("scorsese-unknown-argument/nowhere.scor");
    for tool in registry() {
        let arguments = json!({
            "project": project.to_string_lossy(),
            "not_an_argument_of_any_tool": 1
        });
        let refused = tool.call(&arguments).err().unwrap_or_default();
        assert!(
            refused.starts_with("`not_an_argument_of_any_tool` is not an argument here."),
            "`{}` did not refuse an unknown argument; it said: {refused}",
            tool.name()
        );
    }
    assert!(
        !project.exists(),
        "a tool acted before refusing its arguments"
    );
}

#[test]
fn every_tool_refuses_a_key_its_object_arguments_do_not_take() {
    let project = std::env::temp_dir().join("scorsese-unknown-key/nowhere.scor");
    let mut checked = 0;
    for tool in registry() {
        let schema = tool.schema();
        let Some(properties) = schema["properties"].as_object() else {
            continue;
        };
        for (name, property) in properties {
            let (given, at) = match closed(property) {
                true => (json!({ "not_a_key": 1 }), format!("{name}.not_a_key")),
                false if closed(&property["items"]) => {
                    (json!([{ "not_a_key": 1 }]), format!("{name}[0].not_a_key"))
                }
                false => continue,
            };
            let arguments = json!({ "project": project.to_string_lossy(), name: given });
            let refused = tool.call(&arguments).err().unwrap_or_default();
            assert!(
                refused.starts_with(&format!("`{at}` is not an argument here.")),
                "`{}` did not refuse `{at}`; it said: {refused}",
                tool.name()
            );
            checked += 1;
        }
    }
    // shadow, glow, matte, the briefs, an arrow's ends, keyframes, scenes…
    assert!(checked >= 12, "only {checked} object arguments were found");
    assert!(
        !project.exists(),
        "a tool acted before refusing its arguments"
    );
}

/// Whether a schema is an object that lists the keys it takes, and no others.
fn closed(schema: &serde_json::Value) -> bool {
    schema["properties"].is_object()
        && matches!(
            schema.get("additionalProperties"),
            None | Some(serde_json::Value::Bool(false))
        )
}

#[test]
fn an_arrows_end_takes_the_same_keys_as_its_start() {
    let tool = registry()
        .into_iter()
        .find(|tool| tool.name() == "asset_set")
        .expect("asset_set");
    let schema = tool.schema();
    let keys = |end: &str| {
        let mut keys: Vec<String> = schema["properties"][end]["properties"]
            .as_object()
            .map(|properties| properties.keys().cloned().collect())
            .unwrap_or_default();
        keys.sort();
        keys
    };
    assert_eq!(keys("to"), keys("from"));
    assert_eq!(
        schema["properties"]["to"]["properties"]["side"]["enum"],
        schema["properties"]["from"]["properties"]["side"]["enum"]
    );
}
