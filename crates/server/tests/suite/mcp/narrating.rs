//! A narration's word timings on the web (#886, the web half of #811): kept
//! with the line in the library, laid out beside its audio, named by
//! `project_describe`, and told to every page a render has captured.

use std::path::Path;
use std::time::Duration;

use scorsese_core::{AssetKind, Project};
use scorsese_server::captures::Spool;
use scorsese_server::projects;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::paying::{fund, narrated, quote_and_confirm};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

#[sqlx::test]
async fn a_lines_word_timings_reach_the_assistant_and_the_pages(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, narrated()).await;
    fund(&pool, ana, 10).await;
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);

    // Kept on the line's own row.
    let words: Value =
        sqlx::query_scalar("SELECT words FROM library_items WHERE brief_hash IS NOT NULL")
            .fetch_one(&pool)
            .await
            .expect("the line has a row");
    assert_eq!(words["words"][0]["text"], "Every", "{words}");

    // A page beside the line on the timeline.
    let page = json!({ "project": id, "page": "title", "html": "<p>Hi</p>" });
    let (said, refused) = call(address, &who, "page_write", page).await;
    assert!(!refused, "{said}");
    placed(&pool, ana, id).await;

    // The assistant hears it: the word being said at an instant.
    let asked = json!({ "project": id, "at": "0.1s" });
    let (said, refused) = call(address, &who, "project_describe", asked).await;
    assert!(!refused, "{said}");
    assert!(said.contains("say is saying \"Every\""), "{said}");

    // And the page is told it, in what the render asks the capture worker.
    let spool = state.renders.captures();
    let answering = tokio::task::spawn_blocking(move || answer_one(&spool));
    let asked = json!({ "project": id, "resolution": "64x36" });
    let (said, refused) = call(address, &who, "render", asked).await;
    assert!(!refused, "{said}");
    let ask = answering.await.expect("the worker answered");
    let told = &ask["requests"][0]["words"];
    assert!(told.get("say/every").is_some(), "{ask}");
    assert!(told.get("say/editor").is_some(), "{ask}");
}

/// The line on an audio track as clip `say`, and the page on a video track
/// over it, saved over project `id`.
async fn placed(pool: &PgPool, user: scorsese_server::db::UserId, id: i64) {
    let stored = document(pool, user, id).await;
    let page = stored
        .document
        .assets
        .iter()
        .find(|asset| asset.kind == AssetKind::Html)
        .expect("the page is an asset")
        .id
        .clone();
    let mut edited = serde_json::to_value(&stored.document).expect("a project serialises");
    edited["tracks"] = json!([
        { "id": "v1", "kind": "video",
          "clips": [{ "id": "show", "asset": page.as_str(), "start": 0, "duration": 30 }] },
        { "id": "a1", "kind": "audio",
          "clips": [{ "id": "say", "asset": "vo", "start": 0, "duration": 30 }] }
    ]);
    let edited = Project::from_json(&edited.to_string()).expect("still a project");
    projects::save(pool, user, id, stored.summary.revision, &edited)
        .await
        .expect("the edit is saved");
}

/// The first capture job that appears in `spool`, answered as if every page
/// were captured; what it asked.
fn answer_one(spool: &Spool) -> Value {
    for _ in 0..1200 {
        let asked = std::fs::read_dir(spool.jobs())
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .find_map(|job| read(&job.join("ask.json")).map(|ask| (job, ask)));
        if let Some((job, ask)) = asked {
            let count = ask["requests"].as_array().map_or(0, Vec::len);
            let answer = json!({ "failed": vec![Value::Null; count] });
            let partial = job.join("answer.partial");
            std::fs::write(&partial, answer.to_string()).expect("the answer is written");
            std::fs::rename(&partial, job.join("answer.json")).expect("and published");
            return ask;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the render never asked for its pages");
}

fn read(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}
