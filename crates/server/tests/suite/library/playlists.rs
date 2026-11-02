//! A playlist or a list of files is refused at upload, in plain words, and
//! never opened (#1635): what a member's file names must not decide what
//! else the server reads.

use sqlx::postgres::PgPool;

use crate::common;
use crate::library::{announce, member, patch};

#[sqlx::test]
async fn a_list_uploaded_as_a_video_is_refused_as_a_playlist(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-playlist")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let list = b"ffconcat version 1.0\nfile ../../../secret.mp4\n".to_vec();
    let created = announce(address, &ana, "holiday.mp4", &list).await;
    let location = created.header("location").expect("a location").to_owned();
    let refused = patch(address, &ana, &location, 0, &list).await;
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(
        refused
            .body
            .contains("this file is a playlist, not a video; upload the video itself"),
        "{}",
        refused.body
    );
}
