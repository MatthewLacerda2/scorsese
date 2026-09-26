//! Projects over HTTP: made, opened, saved by revision, renamed, deleted —
//! and never reachable by another user.

mod common;

use std::net::SocketAddr;

use common::request;
use scorsese_server::accounts::{sessions, users};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

/// A logged-in browser's `Cookie:` line for a new account `email`.
async fn member(pool: &PgPool, email: &str) -> String {
    let user = users::create(pool, email, "password one")
        .await
        .expect("the account is created");
    let cookie = sessions::open(pool, user).await.expect("a session opens");
    format!("Cookie: scorsese_session={cookie}")
}

async fn call(
    address: SocketAddr,
    who: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let response = request(address, method, path, &[who], body.as_ref()).await;
    let json = serde_json::from_str(&response.body).unwrap_or(Value::Null);
    (response.status, json)
}

#[sqlx::test]
async fn a_project_is_made_opened_saved_renamed_and_deleted(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = member(&pool, "ana@example.com").await;

    let (status, made) = call(
        address,
        &ana,
        "POST",
        "/api/projects",
        Some(json!({ "name": " teaser " })),
    )
    .await;
    assert_eq!(status, 201, "{made}");
    assert_eq!(made["name"], "teaser");
    assert_eq!(made["document"]["name"], "teaser");
    let path = format!("/api/projects/{}", made["id"]);

    let (_, opened) = call(address, &ana, "GET", &path, None).await;
    assert_eq!(opened["revision"], 1);
    let mut document = opened["document"].clone();
    document["tracks"] = json!([{ "id": "v1", "kind": "video" }]);

    let save = json!({ "revision": 1, "document": document });
    let (status, saved) = call(address, &ana, "PUT", &path, Some(save.clone())).await;
    assert_eq!((status, saved["revision"].clone()), (200, json!(2)));

    // The same save again is based on a revision that has moved on.
    let (status, refused) = call(address, &ana, "PUT", &path, Some(save)).await;
    assert_eq!(status, 409, "{refused}");

    let rename = json!({ "name": "trailer" });
    let (status, renamed) = call(address, &ana, "PATCH", &path, Some(rename)).await;
    assert_eq!((status, renamed["revision"].clone()), (200, json!(3)));
    let (_, listed) = call(address, &ana, "GET", "/api/projects", None).await;
    assert_eq!(listed[0]["name"], "trailer");
    assert!(
        listed[0].get("document").is_none(),
        "a list carries no documents"
    );
    let (_, opened) = call(address, &ana, "GET", &path, None).await;
    assert_eq!(
        opened["document"]["tracks"][0]["id"], "v1",
        "the rename kept the save"
    );

    assert_eq!(call(address, &ana, "DELETE", &path, None).await.0, 204);
    assert_eq!(call(address, &ana, "GET", &path, None).await.0, 404);
}

#[sqlx::test]
async fn a_document_this_build_does_not_read_is_refused(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = member(&pool, "ana@example.com").await;
    let (_, made) = call(
        address,
        &ana,
        "POST",
        "/api/projects",
        Some(json!({ "name": "x" })),
    )
    .await;
    let path = format!("/api/projects/{}", made["id"]);

    for broken in [
        json!({ "schema_version": 1, "name": "old" }),
        json!({ "schema_version": made["document"]["schema_version"], "name": "x",
                "timeline_fps": made["document"]["timeline_fps"], "trackz": [] }),
    ] {
        let body = json!({ "revision": 1, "document": broken });
        let (status, error) = call(address, &ana, "PUT", &path, Some(body)).await;
        assert_eq!(status, 400, "{error}");
    }
    assert_eq!(
        call(address, &ana, "GET", &path, None).await.1["revision"],
        1
    );
}

#[sqlx::test]
async fn one_user_cannot_reach_anothers_projects(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let ana = member(&pool, "ana@example.com").await;
    let bia = member(&pool, "bia@example.com").await;
    let (_, made) = call(
        address,
        &ana,
        "POST",
        "/api/projects",
        Some(json!({ "name": "ana's" })),
    )
    .await;
    let path = format!("/api/projects/{}", made["id"]);

    assert_eq!(
        call(address, &bia, "GET", "/api/projects", None).await.1,
        json!([])
    );
    assert_eq!(call(address, &bia, "GET", &path, None).await.0, 404);
    let save = json!({ "revision": 1, "document": made["document"] });
    assert_eq!(call(address, &bia, "PUT", &path, Some(save)).await.0, 404);
    let rename = json!({ "name": "mine now" });
    assert_eq!(
        call(address, &bia, "PATCH", &path, Some(rename)).await.0,
        404
    );
    assert_eq!(call(address, &bia, "DELETE", &path, None).await.0, 404);

    let (_, still) = call(address, &ana, "GET", &path, None).await;
    assert_eq!(
        (still["name"].clone(), still["revision"].clone()),
        (json!("ana's"), json!(1))
    );
}
