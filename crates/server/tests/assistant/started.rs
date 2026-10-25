//! A project the web app makes (#1531): a name and nothing else. The web asks
//! for no platform, style or shape; the assistant reads them from the first
//! prompt and writes them into the script. The local CLI and MCP, and the web
//! MCP's own `project_new`, still take a platform and a style (#1015).

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, common, member, scripted};

/// `POST /api/projects` with `body`: the status and the answer.
async fn create(address: std::net::SocketAddr, who: &str, body: Value) -> (u16, Value) {
    let made = common::request(address, "POST", "/api/projects", &[who], Some(&body)).await;
    (made.status, made.json())
}

#[sqlx::test]
async fn a_project_is_made_from_a_name_and_nothing_else(pool: PgPool) {
    let (address, _) = scripted(&pool, &Script::new(vec![])).await;
    let (_, who) = member(&pool, "ana@example.com", 10).await;
    // A platform or a style in the body is not the web's to send any more,
    // and starts nothing: no brief, no choice kept.
    let body = json!({ "name": "Ad", "platform": "tiktok_ad", "style": "kinetic_type" });
    let (status, made) = create(address, &who, body).await;
    assert_eq!(status, 201, "{made}");
    assert!(
        made["platform"].is_null() && made["style"].is_null(),
        "{made}"
    );
    assert!(made["document"].get("script").is_none(), "{made}");
}

#[sqlx::test]
async fn a_refused_create_keeps_nothing(pool: PgPool) {
    let (address, _) = scripted(&pool, &Script::new(vec![])).await;
    let (_, who) = member(&pool, "ana@example.com", 10).await;
    for (body, says) in [
        (json!({ "name": "  " }), "needs a name"),
        (json!({ "name": "x", "assets": [999] }), "no file 999"),
    ] {
        let (status, refused) = create(address, &who, body.clone()).await;
        assert_eq!(status, 400, "{body}: {refused}");
        assert!(
            refused["error"].as_str().unwrap().contains(says),
            "{refused}"
        );
    }
    let listed = common::request(address, "GET", "/api/projects", &[&who], None).await;
    assert_eq!(listed.json(), json!([]), "a refused create leaves nothing");
}

#[sqlx::test]
async fn the_start_menu_and_its_change_are_gone(pool: PgPool) {
    let (address, _) = scripted(&pool, &Script::new(vec![])).await;
    let (_, who) = member(&pool, "ana@example.com", 10).await;
    let (_, made) = create(address, &who, json!({ "name": "Ad" })).await;
    let id = made["id"].as_i64().unwrap();
    let menu = common::request(address, "GET", "/api/styles", &[&who], None).await;
    assert_eq!(menu.status, 404, "{}", menu.body);
    let change = json!({ "platform": "tiktok", "style": "top_list" });
    let path = format!("/api/projects/{id}/start");
    let changed = common::request(address, "PUT", &path, &[&who], Some(&change)).await;
    assert_eq!(changed.status, 404, "{}", changed.body);
}
