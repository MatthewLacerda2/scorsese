//! `docs/mcp.md`'s rule on the web: every tool and every argument describes
//! itself — and a registry tool is described in exactly the registry's words.

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{common, member, post};

/// The web's `tools/list`, as a client reads it.
async fn listed(pool: &PgPool) -> Vec<Value> {
    let address = common::serve(pool.clone()).await;
    let (_, ana) = member(pool, "ana@example.com").await;
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
    let reply = post(address, &ana, &body).await.json();
    reply["result"]["tools"]
        .as_array()
        .expect("a list of tools")
        .clone()
}

#[sqlx::test]
async fn every_tool_and_every_argument_says_what_it_is(pool: PgPool) {
    for tool in listed(&pool).await {
        let name = tool["name"].as_str().expect("a name");
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(description.len() > 40, "{name} is not described");
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{name} has no properties object"));
        for (argument, property) in properties {
            let said = property["description"].as_str().unwrap_or_default();
            assert!(said.len() > 15, "{name}.{argument} is not described");
        }
        if properties.contains_key("project") {
            assert_eq!(
                properties["project"]["type"],
                json!("integer"),
                "{name} addresses a project by its id on the web"
            );
        }
    }
}

/// One registry, two ways in: a tool the web serves from the registry says
/// exactly what the stdio server's says, and the web's own tools stand where
/// what they replace stands.
#[sqlx::test]
async fn the_registrys_tools_keep_the_registrys_words(pool: PgPool) {
    let listed = listed(&pool).await;
    let named = |name: &str| listed.iter().find(|tool| tool["name"] == name).cloned();
    for tool in scorsese_mcp::registry() {
        let Some(served) = named(tool.name()) else {
            continue;
        };
        if ["project_new", "import", "render", "generate"].contains(&tool.name()) {
            assert_ne!(served["description"], json!(tool.description()));
            continue;
        }
        assert_eq!(
            served["description"],
            json!(tool.description()),
            "{}",
            tool.name()
        );
    }
    for web in ["project_list", "library", "jobs", "spending_history"] {
        assert!(named(web).is_some(), "{web} is served");
    }
    for withheld in ["synth_new", "script_read", "voice_design"] {
        assert!(named(withheld).is_none(), "{withheld} is not served yet");
    }
    let docs = include_str!("../../../../docs/web.md");
    for tool in &listed {
        let name = tool["name"].as_str().expect("a name");
        assert!(
            docs.contains(&format!("`{name}`")),
            "docs/web.md never names {name}"
        );
    }
}
