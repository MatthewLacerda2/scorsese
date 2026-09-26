//! Generated output in the library: found again by its brief, by its owner
//! only, and carrying the record of what it cost.

use std::path::{Path, PathBuf};

use scorsese_server::credits::generations::{self, Answer, Request, Shot};
use scorsese_server::credits::ledger;
use scorsese_server::db::{self, UserId};
use scorsese_server::events::Events;
use scorsese_server::jobs::Queue;
use scorsese_server::library::{Arrival, Kind, Library};
use sqlx::postgres::PgPool;

use crate::common;
use crate::member;

const BRIEF: &str = "b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0";

/// A spoken line, as a provider would have left it: a wav under `directory`.
fn line(directory: &Path, name: &str) -> PathBuf {
    let file = directory.join(name);
    let output = common::tools()
        .ffmpeg()
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "sine=duration=1"])
        .arg(&file)
        .output()
        .expect("ffmpeg runs");
    assert!(output.status.success(), "{output:?}");
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
    }
}

async fn library(pool: &PgPool) -> Library {
    db::migrate(pool).await.expect("the migrations apply");
    let files = common::files("generated");
    let members = db::member_pool(pool)
        .await
        .expect("the member pool connects");
    Library::new(
        members,
        files.storage,
        files.tools,
        Queue::new(Events::new()),
    )
}

#[sqlx::test]
async fn output_is_found_by_its_brief_and_only_by_its_owner(pool: PgPool) {
    let library = library(&pool).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let (bia, _) = member(&pool, "bia@example.com").await;
    let scratch = common::scratch("generated-found");

    assert!(library.find_generated(ana, BRIEF).await.unwrap().is_none());
    let kept = library
        .keep_generated(ana, BRIEF, arrival(line(&scratch, "a.wav")))
        .await
        .unwrap();
    assert_eq!(kept.brief_hash.as_deref(), Some(BRIEF));
    let found = library.find_generated(ana, BRIEF).await.unwrap();
    assert_eq!(found.map(|item| item.id), Some(kept.id));
    assert!(
        library.find_generated(bia, BRIEF).await.unwrap().is_none(),
        "a brief is never shared across users"
    );

    // Identical output from another brief is the item already there.
    let other = "c".repeat(64);
    let again = library
        .keep_generated(ana, &other, arrival(line(&scratch, "b.wav")))
        .await
        .unwrap();
    assert_eq!(again.id, kept.id);
}

#[sqlx::test]
async fn a_generated_item_carries_the_record_of_what_it_cost(pool: PgPool) {
    let library = library(&pool).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let scratch = common::scratch("generated-record");
    let kept = library
        .keep_generated(ana, BRIEF, arrival(line(&scratch, "a.wav")))
        .await
        .unwrap();
    assert!(library.generation(ana, kept.id).await.unwrap().is_none());

    pay_for_a_shot(&pool, ana, Some(kept.id)).await;
    let record = library.generation(ana, kept.id).await.unwrap().unwrap();
    assert_eq!(record["kind"], "veo_shot");
    assert_eq!(record["prompt"], "a lighthouse at dusk");
    assert_eq!(record["estimated_cost_micros"], 960_000);
    assert_eq!(record["charged_micros"], 1_056_000);

    // Deleting the item keeps the record of the money, and forgets the item.
    library.delete(ana, kept.id).await.unwrap();
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let item: Option<i64> = sqlx::query_scalar("SELECT library_item_id FROM veo_generations")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(item, None);
}

/// A shot funded, reserved for and settled as having made `item`.
async fn pay_for_a_shot(pool: &PgPool, user: UserId, item: Option<i64>) {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    ledger::top_up(&mut tx, 5_000, 50_000)
        .await
        .expect("a top-up is recorded");
    let shot = Request::Shot(Shot {
        project: None,
        tool_call: None,
        job: None,
        model: "fast",
        resolution: "1080p",
        seconds: 8,
        aspect: "16:9",
        prompt: "a lighthouse at dusk",
        brief_hash: BRIEF,
        estimated_cents: 96,
    });
    let paid = generations::start(&mut tx, &shot)
        .await
        .expect("ten dollars cover a shot");
    generations::finish(&mut tx, paid, &Answer::Worked(item))
        .await
        .expect("the shot settles");
    tx.commit().await.expect("the shot commits");
}
