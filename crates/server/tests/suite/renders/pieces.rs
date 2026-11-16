//! A preview played in pieces (#1448): each piece a fragmented MP4 of its
//! range, carrying FLAC sound whenever the cut has any — silence where its
//! own stretch has none — and saying so before it is made; and a piece of an
//! older revision is stopped by any piece of a newer one, whatever its range.

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

/// The codecs in render `id`'s file, and whether it is fragmented.
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
        .args(["-v", "error", "-show_entries", "stream=codec_name"])
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

/// The card with a tone under its first three frames, the tone a file in
/// `user`'s library.
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
                "clips": [{ "id": "t1", "asset": "tone", "start": 0, "duration": 3 }]
            }));
    })
}

/// Ask for the piece of project `id` over `range` and wait for it: whether
/// the answer said the cut is audible, and the piece's codecs.
async fn piece(
    pool: &PgPool,
    files: &Files,
    (address, cookie, user): (std::net::SocketAddr, &str, UserId),
    id: i64,
    range: &str,
) -> (Value, Vec<String>) {
    let path = format!("/api/projects/{id}/previews");
    let ask = json!({ "resolution": "64x36", "range": range });
    let (status, asked) = call(address, cookie, "POST", &path, Some(ask)).await;
    assert_eq!(status, 202, "{asked}");
    let job = finished(pool, user, job_of(&asked)).await;
    assert_eq!(job.state, State::Done, "{:?}", job.error);
    let (codecs, fragmented) = streams(
        pool,
        files,
        &job.result.expect("a done job has a result")["render"],
    )
    .await;
    assert!(fragmented, "{range}: a piece is fragmented");
    (asked["audible"].clone(), codecs)
}

#[sqlx::test]
async fn every_piece_of_a_cut_with_sound_carries_sound_and_says_so_first(pool: PgPool) {
    let files = common::files("preview-pieces");
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let who = (address, cookie.as_str(), ana);
    let toned = stored(&pool, ana, &with_tone(&pool, &files, ana).await).await;
    let both = vec!["h264".to_owned(), "flac".to_owned()];
    // The tone, and then the stretch after it, silent but still with sound.
    assert_eq!(
        piece(&pool, &files, who, toned, "0:3").await,
        (json!(true), both.clone())
    );
    assert_eq!(
        piece(&pool, &files, who, toned, "3:").await,
        (json!(true), both)
    );

    let quiet = stored(&pool, ana, &card(|_| {})).await;
    let only = vec!["h264".to_owned()];
    assert_eq!(
        piece(&pool, &files, who, quiet, "0:3").await,
        (json!(false), only)
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
    let piece = |range: &str| Some(json!({ "resolution": "64x36", "range": range }));
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
