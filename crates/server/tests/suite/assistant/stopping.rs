//! A Stop reaches the tool call the turn is running (#1469): a `still`
//! waiting on its pages stops waiting at once, rather than for its minute and
//! a half of patience.

use std::time::{Duration, Instant};

use scorsese_core::Project;
use scorsese_server::projects;
use scorsese_server::tools::Client;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, finished, member, scripted, send};

#[sqlx::test]
async fn a_stop_stops_a_still_waiting_on_its_pages(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, state) = scripted(&pool, &script).await;
    state.renders.prepare().expect("the cache and its spool");
    let spool = state.renders.captures();
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let mut document = serde_json::to_value(Project::new("Intro", Default::default())).unwrap();
    document["assets"] = json!([{ "id": "title", "kind": "html", "path": "pages/title.html" }]);
    document["tracks"] = json!([{ "id": "v1", "kind": "video", "clips": [
        { "id": "c", "asset": "title", "start": 0, "duration": 30 }
    ]}]);
    let document = Project::from_json(&document.to_string()).unwrap();
    let id = projects::create(&pool, ana, &document).await.unwrap().id;
    let page = json!({ "project": id, "page": "title", "html": "<body></body>" });
    let client = Client::External { token: None };
    let written = state.tools.call(ana, client, "page_write", &page).await;
    assert!(written.is_ok(), "{:?}", written.err());
    // A worker that says it is there and never answers.
    let alive = spool.root().join("worker.alive");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    std::fs::write(&alive, now.unwrap().as_secs().to_string()).unwrap();
    let look = json!({ "project": id, "at": "0.5s", "resolution": "64x36" });
    script.replace(vec![calls("still", look), answers("never reached")]);

    let sent = send(address, &who, id, "look at the title").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().unwrap();
    let asked = Instant::now();
    while !std::fs::read_dir(spool.jobs()).is_ok_and(|mut jobs| jobs.next().is_some()) {
        assert!(asked.elapsed() < Duration::from_secs(20), "the still asks");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let path = format!("/api/chat/turns/{turn}/stop");
    let stopping = common::request(address, "POST", &path, &[&who], None).await;
    assert_eq!(stopping.status, 202, "{}", stopping.body);

    // Within `finished`'s few seconds, not the still's ninety.
    let detail = finished(address, &who, turn).await;
    assert_eq!(detail["turn"]["state"], "stopped", "{detail}");
    assert_eq!(
        script.requests().len(),
        1,
        "stopped before the model saw it"
    );
}
