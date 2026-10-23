//! A text document is a library kind (#1510) that is never an asset: a `.txt`
//! or `.md` kept as text, at most 1 MB, which only its owner's assistant can
//! read (`library_read`) and which `import` refuses.

use scorsese_core::Project;
use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use scorsese_server::projects;
use scorsese_server::tools::Client;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use crate::common::{self, request, send};
use crate::{announce, member, upload};

const SCRIPT: &[u8] = "# Beach day\n\nScene 1 — waves, and the line “here we go”.\n".as_bytes();

/// Call `name` for `user` as web MCP would; the reply's words, or the refusal.
async fn call(
    state: &AppState,
    user: UserId,
    name: &str,
    arguments: Value,
) -> Result<String, String> {
    state
        .tools
        .call(user, Client::External { token: None }, name, &arguments)
        .await
        .map(|reply| reply.parts[0].text.clone())
}

#[sqlx::test]
async fn a_txt_and_an_md_are_kept_as_text_documents(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-docs")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let notes = upload(address, &ana, "notes.txt", b"shot list: sunrise, coffee").await;
    let script = upload(address, &ana, "Script.MD", SCRIPT).await;

    let listed = request(address, "GET", "/api/library?kind=text", &[&ana], None).await;
    let tiles = listed.json();
    let kinds: Vec<&str> = tiles
        .as_array()
        .unwrap()
        .iter()
        .map(|tile| tile["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["text", "text"], "{}", listed.body);

    for id in [notes, script] {
        let thumbnail = format!("/api/library/{id}/thumbnail");
        let none = request(address, "GET", &thumbnail, &[&ana], None).await;
        assert_eq!(none.status, 404, "{}", none.body);
        assert_eq!(none.json()["pending"], false, "{}", none.body);
    }
    let served = send(
        address,
        "GET",
        &format!("/api/library/{script}/file"),
        &[&ana],
        &[],
    )
    .await;
    assert_eq!(served.status, 200, "{}", served.body);
    assert_eq!(
        served.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(served.body.as_bytes(), SCRIPT);
}

#[sqlx::test]
async fn a_document_that_is_not_utf8_or_over_1_mb_is_refused(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("refuse-docs")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let large = vec![b'a'; 1_000_001];
    let refused = announce(address, &ana, "long.md", &large).await;
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(
        refused.body.contains("a document can be at most 1.0 MB"),
        "{}",
        refused.body
    );
    let fits = announce(address, &ana, "full.md", &large[1..]).await;
    assert_eq!(fits.status, 201, "1 MB exactly is let in: {}", fits.body);

    let latin1: &[u8] = b"caf\xe9 au lait";
    let created = announce(address, &ana, "menu.txt", latin1).await;
    assert_eq!(created.status, 201, "{}", created.body);
    let location = created.header("location").expect("a Location").to_owned();
    let last = crate::patch(address, &ana, &location, 0, latin1).await;
    assert_eq!(last.status, 422, "{}", last.body);
    assert!(
        last.body
            .contains("this is not a document: it has to be text (UTF-8)"),
        "{}",
        last.body
    );
}

#[sqlx::test]
async fn a_document_reaches_its_owners_assistant_alone(pool: PgPool) {
    let (address, state) = common::serve_with(pool.clone(), common::files("read-docs")).await;
    let (ana, ana_cookie) = member(&pool, "ana@example.com").await;
    let (bob, _) = member(&pool, "bob@example.com").await;
    let script = upload(address, &ana_cookie, "script.md", SCRIPT).await;

    let listed = call(&state, ana, "library", json!({ "kind": "text" }))
        .await
        .unwrap();
    assert!(
        listed.contains(&format!("{script} — script.md (text)")),
        "{listed}"
    );
    let read = call(&state, ana, "library_read", json!({ "item": script }))
        .await
        .unwrap();
    assert!(
        read.ends_with(std::str::from_utf8(SCRIPT).unwrap()),
        "{read}"
    );

    let refused = call(&state, bob, "library_read", json!({ "item": script }))
        .await
        .unwrap_err();
    assert_eq!(
        refused,
        format!("there is no file {script} in your library")
    );
    let theirs = call(&state, bob, "library", json!({})).await.unwrap();
    assert!(!theirs.contains("script.md"), "{theirs}");
}

#[sqlx::test]
async fn import_refuses_a_document_and_library_read_refuses_media(pool: PgPool) {
    let (address, state) = common::serve_with(pool.clone(), common::files("import-docs")).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let script = upload(address, &cookie, "script.txt", SCRIPT).await;
    let page = upload(address, &cookie, "card.html", b"<p>hi</p>").await;
    let project = projects::create(&pool, ana, &Project::new("Beach", Default::default()))
        .await
        .expect("stored")
        .id;

    let arguments = json!({ "project": project, "items": [script] });
    let refused = call(&state, ana, "import", arguments).await.unwrap_err();
    assert!(
        refused.contains("is a document, which is never imported"),
        "{refused}"
    );
    let stored = projects::open(&pool, ana, project).await.unwrap();
    assert!(stored.document.assets.is_empty());

    let refused = call(&state, ana, "library_read", json!({ "item": page }))
        .await
        .unwrap_err();
    assert!(refused.contains("is a page, not a document"), "{refused}");
}
