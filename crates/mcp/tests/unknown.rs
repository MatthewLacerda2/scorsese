//! Every tool refuses an argument it does not take (#1117).
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
