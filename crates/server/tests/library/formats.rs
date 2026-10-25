//! What the library takes, as the browser asks it (#1534): the extensions
//! each kind is uploaded under, from the same list an upload is held to, so
//! the web app's file pickers and drops never keep a second one. Asked
//! signed out too: the landing's prompt box holds files before sign-in.

use sqlx::postgres::PgPool;

use crate::common::{self, request};

#[sqlx::test]
async fn the_formats_are_listed_by_kind_signed_out(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("formats")).await;
    let listed = request(address, "GET", "/api/library/formats", &[], None).await;
    assert_eq!(listed.status, 200, "{}", listed.body);
    let formats = listed.json();
    let has = |kind: &str, extension: &str| {
        formats[kind]
            .as_array()
            .is_some_and(|all| all.iter().any(|one| one == extension))
    };
    assert!(has("video", "mp4") && has("video", "mov"), "{formats}");
    assert!(has("image", "png") && has("audio", "mp3"), "{formats}");
    assert!(has("html", "html") && has("text", "md"), "{formats}");
    assert!(
        !has("audio", "mid"),
        "MIDI is not a library kind: {formats}"
    );
}
