//! A stored project never names a file outside its folder: a save that would
//! is refused in validation's words, and a paid generation is never linked
//! under a name its asset's id makes climb out.

use crate::common;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use common::request;
use scorsese_core::{Fps, Project};
use scorsese_providers::speech;
use scorsese_render::testing::generate;
use scorsese_server::accounts::{sessions, users};
use scorsese_server::db::{self, UserId};
use scorsese_server::events::Events;
use scorsese_server::jobs::Queue;
use scorsese_server::library::{Arrival, Kind, Library};
use scorsese_server::projects::ProjectFiles;
use scorsese_server::tools::lay_out;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

/// A new account, and a logged-in browser's `Cookie:` line for it.
async fn member(pool: &PgPool, email: &str) -> (UserId, String) {
    let user = users::create(pool, email)
        .await
        .expect("the account is created");
    let cookie = sessions::open(pool, user).await.expect("a session opens");
    (user, format!("Cookie: scorsese_session={cookie}"))
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

/// A spoken line, as a provider would have left it: a wav under `directory`.
fn line(directory: &Path, name: &str) -> PathBuf {
    let file = directory.join(name);
    generate(
        &common::tools(),
        &file,
        &["-f", "lavfi", "-i", "sine=duration=1"],
    );
    file
}

fn arrival(file: PathBuf) -> Arrival {
    Arrival {
        file,
        name: "narration".to_owned(),
        kind: Kind::Audio,
        extension: "wav".to_owned(),
        announced: None,
        brief_hash: None,
        words: None,
    }
}

/// A save is validated as `project_write` validates locally: a path out of the
/// project is refused in validation's own words, hashed or not, and nothing is
/// stored.
#[sqlx::test]
async fn a_document_that_does_not_validate_is_refused_and_not_stored(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, made) = call(
        address,
        &ana,
        "POST",
        "/api/projects",
        Some(json!({ "name": "x" })),
    )
    .await;
    let path = format!("/api/projects/{}", made["id"]);

    let mut document = made["document"].clone();
    document["assets"] =
        json!([{ "id": "leak", "kind": "image", "path": "assets/../../outside.png" }]);
    let body = json!({ "revision": 1, "document": document });
    let (status, error) = call(address, &ana, "PUT", &path, Some(body)).await;
    assert_eq!(status, 400, "{error}");
    let said = error.to_string();
    for says in [
        "leak",
        "assets/../../outside.png",
        "escapes the project root",
    ] {
        assert!(said.contains(says), "{says}: {said}");
    }
    assert_eq!(
        call(address, &ana, "GET", &path, None).await.1["revision"],
        1
    );
}

/// A paid line is linked into a laid-out folder under a name made from its
/// asset's id, and an id is whatever the document says: one that climbs never
/// puts a link outside the folder.
#[sqlx::test]
async fn a_paid_line_whose_id_climbs_is_never_linked_outside_the_folder(pool: PgPool) {
    db::migrate(&pool).await.expect("the migrations apply");
    let files = common::files("generated-climbs");
    let members = common::members(&pool).await;
    let storage = files.storage.clone();
    let library = Library::new(
        members,
        files.storage,
        files.tools,
        Queue::new(Events::new()),
    );
    let (ana, _) = member(&pool, "ana@example.com").await;
    let mut project = Project::new("p", Fps::default());
    project.assets = vec![
        serde_json::from_value(serde_json::json!({
            "id": "../../escaped", "kind": "generated_audio", "prompt": "hello there",
            "state": "sketch", "speech": { "voice_id": "voice" }
        }))
        .unwrap(),
    ];
    let brief = speech::Brief::of(&project.assets[0]).unwrap();
    let scratch = common::scratch::dir("generated-climbs-line");
    library
        .keep_generated(ana, &brief.digest(), arrival(line(&scratch, "a.wav")))
        .await
        .unwrap();

    let folder = lay_out(&pool, &storage, ana, &project, &ProjectFiles::default()).await;
    let root = storage.scratch(ana);
    let beside: Vec<_> = std::fs::read_dir(root.parent().unwrap())
        .into_iter()
        .flatten()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| !name.to_string_lossy().ends_with(".scor"))
        .collect();
    assert!(beside.is_empty(), "linked outside the folder: {beside:?}");
    drop(folder);
}
