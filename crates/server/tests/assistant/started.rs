//! A project started for a platform and a style (#1016): made over HTTP with
//! its brief in the script, offered from `GET /api/styles`, and changed later
//! by a turn whose server note says what changed — the script left to the
//! assistant.

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, common, finished, member, scripted};

/// `POST /api/projects` with `body`: the status and the answer.
async fn create(address: std::net::SocketAddr, who: &str, body: Value) -> (u16, Value) {
    let made = common::request(address, "POST", "/api/projects", &[who], Some(&body)).await;
    (made.status, made.json())
}

/// The text project `id` keeps at `path`, read past its owner's policy.
async fn kept(pool: &PgPool, id: i64, path: &str) -> Option<String> {
    sqlx::query_scalar("SELECT content FROM project_files WHERE project_id = $1 AND path = $2")
        .bind(id)
        .bind(path)
        .fetch_optional(pool)
        .await
        .expect("the files can be read")
}

#[sqlx::test]
async fn a_started_project_carries_its_brief_and_its_choice(pool: PgPool) {
    let (address, _) = scripted(&pool, &Script::new(vec![])).await;
    let (_, who) = member(&pool, "ana@example.com", 10).await;
    let body = json!({ "name": "Ad", "platform": "tiktok_ad", "style": "kinetic_type" });
    let (status, made) = create(address, &who, body).await;
    assert_eq!(status, 201, "{made}");
    assert_eq!(made["platform"], "tiktok_ad");
    assert_eq!(made["style"], "kinetic_type");
    assert_eq!(made["document"]["script"], "script.md");
    let id = made["id"].as_i64().unwrap();
    let brief = kept(&pool, id, "script.md")
        .await
        .expect("the brief is kept");
    assert!(brief.contains("Platform: Anúncio no TikTok"), "{brief}");
    assert!(brief.contains("Kinetic typography"), "{brief}");
    assert!(brief.contains("propose the script"), "{brief}");

    // Nothing chosen is a plain project, with no script at all.
    let (_, plain) = create(address, &who, json!({ "name": "Plain" })).await;
    assert!(
        plain["platform"].is_null() && plain["style"].is_null(),
        "{plain}"
    );
    assert!(plain["document"].get("script").is_none(), "{plain}");
}

#[sqlx::test]
async fn a_refused_start_keeps_nothing(pool: PgPool) {
    let (address, _) = scripted(&pool, &Script::new(vec![])).await;
    let (_, who) = member(&pool, "ana@example.com", 10).await;
    for (body, says) in [
        (
            json!({ "name": "x", "platform": "myspace" }),
            "youtube_shorts",
        ),
        (json!({ "name": "x", "style": "nope" }), "kinetic_type"),
        (
            json!({ "name": "x", "platform": "youtube", "style": "flash_offer" }),
            "not made for",
        ),
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
async fn the_menu_lists_every_platform_and_style(pool: PgPool) {
    let (address, _) = scripted(&pool, &Script::new(vec![])).await;
    let (_, who) = member(&pool, "ana@example.com", 10).await;
    let menu = common::request(address, "GET", "/api/styles", &[&who], None)
        .await
        .json();
    let platforms = menu["platforms"].as_array().unwrap();
    assert_eq!(platforms.len(), 7, "{menu}");
    assert_eq!(platforms[1]["id"], "youtube_shorts");
    assert_eq!(
        (&platforms[1]["width"], &platforms[1]["height"]),
        (&json!(1080), &json!(1920))
    );
    let style = &menu["styles"][0];
    assert_eq!(style["id"], "narrated_captions");
    assert!(style["preview"].is_null(), "no previews until #1017");
    assert!(
        style["platforms"]
            .as_array()
            .unwrap()
            .contains(&json!("tiktok"))
    );
    let stranger = common::request(address, "GET", "/api/styles", &[], None).await;
    assert_eq!(stranger.status, 401);
}

#[sqlx::test]
async fn a_change_of_style_is_a_turn_with_a_note_and_the_script_is_left_alone(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (_, who) = member(&pool, "ana@example.com", 10).await;
    let body = json!({ "name": "Ad", "platform": "tiktok", "style": "kinetic_type" });
    let (_, made) = create(address, &who, body).await;
    let id = made["id"].as_i64().unwrap();
    let path = format!("/api/projects/{id}/start");
    let before = kept(&pool, id, "script.md").await;

    script.replace(vec![answers("Here is what changes in the scenes.")]);
    let change = json!({ "platform": "tiktok", "style": "top_list", "message": "Mudei o estilo." });
    let changed = common::request(address, "PUT", &path, &[&who], Some(&change)).await;
    assert_eq!(changed.status, 200, "{}", changed.body);
    let turn = changed.json()["turn"]["id"]
        .as_i64()
        .expect("a turn started");
    let detail = finished(address, &who, turn).await;
    assert_eq!(detail["turn"]["prompt"], "Mudei o estilo.");
    let sent = script.messages(0).join("\n");
    assert!(
        sent.contains("the style from Tipografia em movimento to Lista / Top N"),
        "{sent}"
    );
    assert!(sent.contains("script_write"), "{sent}");
    assert!(
        sent.contains("Top N list"),
        "the new style's prompt: {sent}"
    );
    assert_eq!(
        kept(&pool, id, "script.md").await,
        before,
        "the server rewrote nothing"
    );
    let project = format!("/api/projects/{id}");
    let opened = common::request(address, "GET", &project, &[&who], None).await;
    assert_eq!(opened.json()["style"], "top_list");

    // The same choice again changes nothing, and starts nothing.
    let same = common::request(address, "PUT", &path, &[&who], Some(&change)).await;
    assert_eq!(same.json(), json!({ "turn": null, "note": null }));
    let unsuited = json!({ "platform": "youtube", "style": "flash_offer" });
    let refused = common::request(address, "PUT", &path, &[&who], Some(&unsuited)).await;
    assert_eq!(refused.status, 400, "{}", refused.body);
}
