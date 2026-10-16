//! A render that asks for no size is delivered at the project's own frame at
//! delivery size (#1278), and one that asks for a size gets it.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, card, common, member, stored};

#[sqlx::test]
async fn no_size_asked_is_the_projects_own_frame(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let id = stored(
        &pool,
        ana,
        &card(|document| document["aspect"] = json!("9:16")),
    )
    .await;
    let renders = format!("/api/projects/{id}/renders");
    let previews = format!("/api/projects/{id}/previews");
    for (path, body) in [
        (&renders, json!({})),
        (&renders, json!({ "resolution": "1920x1080" })),
        (&renders, json!({ "container": "mp3" })),
        (&previews, json!({ "quality": "full" })),
    ] {
        let (status, asked) = call(address, &cookie, "POST", path, Some(body.clone())).await;
        assert_eq!(status, 202, "{body}: {asked}");
    }

    let sizes: Vec<String> = sqlx::query_scalar("SELECT payload::text FROM jobs ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(sizes[0].contains("1080x1920"), "{}", sizes[0]);
    assert!(
        sizes[1].contains("1920x1080"),
        "an asked size is kept: {}",
        sizes[1]
    );
    assert!(
        !sizes[2].contains("1080x1920"),
        "sound only takes no size: {}",
        sizes[2]
    );
    assert!(
        sizes[3].contains("1080x1920"),
        "the preview too: {}",
        sizes[3]
    );
}
