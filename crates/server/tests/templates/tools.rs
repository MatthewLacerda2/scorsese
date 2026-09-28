//! Saving, listing and inserting through the tools, for their owner alone.

use serde_json::json;
use sqlx::postgres::PgPool;

use crate::{call, common, empty, episode, member, only_template, save_intro};
use scorsese_server::projects;

#[sqlx::test]
async fn an_intro_saved_from_one_episode_opens_the_next(pool: PgPool) {
    let (_, state) = common::serve_with(pool.clone(), common::files("templates-use")).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let first = episode(&pool, ana).await;
    let said = save_intro(&state, ana, first, "Intro")
        .await
        .expect("saved");
    assert!(said.contains("Intro") && said.contains("4.0s"), "{said}");

    let listed = call(&state, ana, "template_list", json!({})).await.unwrap();
    assert!(
        listed.contains("Intro (4.0s, 2 clips on 2 tracks: shot, title)"),
        "{listed}"
    );

    let next = empty(&pool, ana).await;
    let template = only_template(&state, ana).await;
    let arguments = json!({ "project": next, "template": template, "at_seconds": 0 });
    let said = call(&state, ana, "template_insert", arguments)
        .await
        .expect("inserted");
    assert!(
        said.contains("c-shot on v1") && said.contains("c-title on v2"),
        "{said}"
    );

    let stored = projects::open(&pool, ana, next).await.unwrap();
    let document = serde_json::to_value(&stored.document).unwrap();
    assert_eq!(document["tracks"][1]["clips"][0]["start"], 30);
    assert_eq!(document["assets"][0]["sha256"], crate::SHOT);
    assert_eq!(stored.summary.revision, 2);
}

/// A second save under a taken name is refused, whatever its case — unless the
/// caller says to replace it, which is how a template is updated.
#[sqlx::test]
async fn a_name_is_one_template_and_replacing_it_is_asked_for(pool: PgPool) {
    let (_, state) = common::serve_with(pool.clone(), common::files("templates-name")).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = episode(&pool, ana).await;
    save_intro(&state, ana, project, "Intro")
        .await
        .expect("saved");

    let again = save_intro(&state, ana, project, "intro").await;
    assert!(
        again
            .as_ref()
            .is_err_and(|why| why.contains("already have")),
        "{again:?}"
    );

    let title_only = json!({ "project": project, "clips": ["c-title"], "name": "INTRO",
                             "replace": true });
    call(&state, ana, "template_save", title_only)
        .await
        .expect("replaced");
    let listed = call(&state, ana, "template_list", json!({})).await.unwrap();
    assert!(listed.contains("1 clips on 1 tracks"), "{listed}");
    assert_eq!(listed.lines().count(), 1);
}

#[sqlx::test]
async fn a_clip_that_is_not_there_saves_nothing(pool: PgPool) {
    let (_, state) = common::serve_with(pool.clone(), common::files("templates-miss")).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = episode(&pool, ana).await;
    let arguments = json!({ "project": project, "clips": ["c-gone"], "name": "Nothing" });
    let refused = call(&state, ana, "template_save", arguments).await;
    assert!(
        refused.as_ref().is_err_and(|why| why.contains("c-gone")),
        "{refused:?}"
    );
    let listed = call(&state, ana, "template_list", json!({})).await.unwrap();
    assert!(listed.contains("no templates"), "{listed}");
}

/// Somebody else's template is no template at all, whatever its id.
#[sqlx::test]
async fn a_template_is_its_owners_alone(pool: PgPool) {
    let (_, state) = common::serve_with(pool.clone(), common::files("templates-own")).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let (bob, _) = member(&pool, "bob@example.com").await;
    save_intro(&state, ana, episode(&pool, ana).await, "Intro")
        .await
        .expect("saved");
    let template = only_template(&state, ana).await;

    let listed = call(&state, bob, "template_list", json!({})).await.unwrap();
    assert!(listed.contains("no templates"), "{listed}");
    let his = empty(&pool, bob).await;
    let arguments = json!({ "project": his, "template": template, "at_seconds": 0 });
    let refused = call(&state, bob, "template_insert", arguments).await;
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.contains("no such template")),
        "{refused:?}"
    );
    assert_eq!(
        projects::open(&pool, bob, his)
            .await
            .unwrap()
            .summary
            .revision,
        1
    );
}
