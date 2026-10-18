//! A preview played in pieces (#1448): a picture piece over its range and the
//! timeline's sound, each a fragmented MP4 of its one stream; a cut with
//! nothing to hear says so without a job; and a piece of an older revision is
//! stopped by any piece of a newer one, whatever its range.

use std::path::Path;

use scorsese_core::{Project, hash_bytes};
use scorsese_render::testing::generate;
use scorsese_server::Files;
use scorsese_server::db::UserId;
use scorsese_server::jobs::State;
use scorsese_server::projects;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::cancelled::{job_of, long, when, worker as on_queue};
use super::job::finished;
use super::previews::worker;
use super::{call, card, common, member, stored};

/// The kinds of stream in render `id`'s file, and whether it is fragmented.
async fn streams(pool: &PgPool, files: &Files, id: &Value) -> (Vec<String>, bool) {
    let path: String = sqlx::query_scalar("SELECT path FROM renders WHERE id = $1")
        .bind(id.as_i64().expect("a render id"))
        .fetch_one(pool)
        .await
        .expect("the render is recorded");
    let file = files.renders.absolute(Path::new(&path));
    let output = files
        .tools
        .ffprobe()
        .args(["-v", "error", "-show_entries", "stream=codec_type"])
        .args(["-of", "csv=p=0"])
        .arg(&file)
        .output()
        .expect("run ffprobe");
    let kinds = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    let bytes = std::fs::read(&file).expect("the file reads");
    (kinds, bytes.windows(4).any(|window| window == b"moof"))
}

/// The card with a second of tone under it, the tone a file in `user`'s
/// library.
async fn with_tone(pool: &PgPool, files: &Files, user: UserId) -> Project {
    let setup = "the test setup works";
    let scratch = common::scratch::dir("tone").join("tone.wav");
    let tone = ["-f", "lavfi", "-i", "sine=frequency=440:duration=1"];
    generate(&files.tools, &scratch, &tone);
    let sha = hash_bytes(&std::fs::read(&scratch).expect(setup));
    let home = files.storage.library_file(user, &sha, "wav");
    std::fs::create_dir_all(home.parent().expect(setup)).expect(setup);
    std::fs::rename(&scratch, home).expect(setup);
    let media = json!({ "duration_seconds": 1.0, "audio_channels": 1 });
    sqlx::query(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'tone', 'audio', 'wav', 1, $3)",
    )
    .bind(user.get())
    .bind(&sha)
    .bind(&media)
    .execute(pool)
    .await
    .expect(setup);
    card(|document| {
        let assets = document["assets"].as_array_mut().expect("assets");
        assets.push(json!({ "id": "tone", "kind": "audio",
            "path": format!("assets/{sha}.wav"), "sha256": sha, "media": media }));
        document["tracks"]
            .as_array_mut()
            .expect("tracks")
            .push(json!({
                "id": "a1", "kind": "audio",
                "clips": [{ "id": "t1", "asset": "tone", "start": 0, "duration": 6 }]
            }));
    })
}

#[sqlx::test]
async fn each_stream_is_its_own_fragmented_file_and_silence_needs_no_job(pool: PgPool) {
    let files = common::files("preview-pieces");
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &with_tone(&pool, &files, ana).await).await;
    let path = format!("/api/projects/{id}/previews");
    for (ask, kind) in [
        (json!({ "stream": "picture", "range": "2:4" }), "video"),
        (json!({ "stream": "sound" }), "audio"),
    ] {
        let mut ask = ask;
        ask["resolution"] = json!("64x36");
        let (status, asked) = call(address, &cookie, "POST", &path, Some(ask)).await;
        assert_eq!(status, 202, "{asked}");
        let job = finished(&pool, ana, job_of(&asked)).await;
        assert_eq!(job.state, State::Done, "{:?}", job.error);
        let render = &job.result.unwrap()["render"];
        assert_eq!(
            streams(&pool, &files, render).await,
            (vec![kind.to_owned()], true)
        );
    }

    let quiet = stored(&pool, ana, &card(|_| {})).await;
    let sound = Some(json!({ "stream": "sound" }));
    let quiet_path = format!("/api/projects/{quiet}/previews");
    let (status, asked) = call(address, &cookie, "POST", &quiet_path, sound).await;
    assert_eq!(status, 200, "{asked}");
    assert_eq!(
        asked,
        json!({ "render": null, "job": null, "silent": true })
    );

    let ranged = Some(json!({ "range": "0:2" }));
    let (status, refused) = call(address, &cookie, "POST", &path, ranged).await;
    assert_eq!(
        status, 400,
        "a range only goes with a picture piece: {refused}"
    );
}

#[sqlx::test]
async fn a_piece_of_a_newer_revision_stops_one_of_an_older_whatever_its_range(pool: PgPool) {
    let files = common::files("preview-pieces-outrun");
    let (address, state) = common::serve_with(pool.clone(), files.clone()).await;
    let _worker = on_queue(&pool, &files, &state).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, &long(36_000)).await;
    let path = format!("/api/projects/{id}/previews");
    let piece =
        |range: &str| Some(json!({ "resolution": "64x36", "stream": "picture", "range": range }));
    let (_, first) = call(address, &cookie, "POST", &path, piece("0:30000")).await;
    let first = job_of(&first);
    when(&pool, ana, first, State::Running).await;
    // Of the same revision, another range waits its turn and stops nothing.
    let (_, next) = call(address, &cookie, "POST", &path, piece("30000:")).await;
    let next = job_of(&next);
    assert_eq!(
        when(&pool, ana, next, State::Waiting).await.state,
        State::Waiting
    );

    let edited = long(36_001);
    projects::save(&pool, ana, id, 1, &edited).await.unwrap();
    let (_, newer) = call(address, &cookie, "POST", &path, piece("0:2")).await;
    when(&pool, ana, first, State::Cancelled).await;
    when(&pool, ana, job_of(&newer), State::Done).await;
}
