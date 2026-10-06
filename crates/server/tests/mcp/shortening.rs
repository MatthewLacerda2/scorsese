//! A line that comes out shorter than the clip laid out over its sketch: the
//! clip is shortened to it, said, and the project still opens (#825).

use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::{fund, quote_and_confirm};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

#[sqlx::test]
async fn a_reading_shorter_than_its_clip_shortens_the_clip(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    // Three seconds laid out for a line the mock reads in one.
    let narrated = json!({
        "assets": [
            { "id": "vo", "kind": "generated_audio", "state": "sketch",
              "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } }
        ],
        "tracks": [ { "id": "a1", "kind": "audio", "clips": [
            { "id": "c-narr", "asset": "vo", "start": 30, "duration": 90 }
        ] } ]
    });
    let id = super::stored(&pool, ana, narrated).await;
    fund(&pool, ana, 10).await;

    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);

    let stored = document(&pool, ana, id).await.document;
    stored.validate().expect("the project still opens");
    let length = stored.assets[0]
        .length(stored.timeline_fps)
        .expect("the line was measured");
    let clip = &stored.tracks[0].clips[0];
    assert!(length.get() < 90, "the mock reads for a second: {length}");
    assert_eq!((clip.start.get(), clip.duration), (30, length));

    let (jobs, _) = call(address, &who, "jobs", json!({})).await;
    assert!(
        jobs.contains(&format!("c-narr: 90f → {length}")) && jobs.contains("came out shorter"),
        "{jobs}"
    );
}
