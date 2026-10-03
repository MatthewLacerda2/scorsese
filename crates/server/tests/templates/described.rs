//! What a template is for (#560): said when it is saved, shown when the
//! templates are listed, and kept when the template is replaced without one.

use serde_json::json;
use sqlx::postgres::PgPool;

use crate::{call, common, episode, member};

#[sqlx::test]
async fn a_description_is_listed_and_survives_a_replace_that_says_none(pool: PgPool) {
    let (_, state) = common::serve_with(pool.clone(), common::files("templates-said")).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = episode(&pool, ana).await;
    let save = |clips: Vec<&str>, description: Option<&str>| {
        let mut arguments =
            json!({ "project": project, "clips": clips, "name": "Intro", "replace": true });
        if let Some(said) = description {
            arguments["description"] = json!(said);
        }
        call(&state, ana, "template_save", arguments)
    };

    let said = save(
        vec!["c-shot", "c-title"],
        Some("  opens every daily video \n"),
    )
    .await
    .expect("saved");
    assert!(said.contains("for: opens every daily video"), "{said}");
    let listed = call(&state, ana, "template_list", json!({})).await.unwrap();
    assert!(
        listed.ends_with("\n    for: opens every daily video"),
        "{listed}"
    );

    save(vec!["c-title"], None).await.expect("replaced");
    let kept = scorsese_server::templates::list(&state.pool, ana)
        .await
        .unwrap();
    assert_eq!(kept[0].clips, 1, "the clips were replaced");
    assert_eq!(
        kept[0].description.as_ref().map(|said| said.as_str()),
        Some("opens every daily video")
    );

    save(vec!["c-title"], Some("the title card alone"))
        .await
        .expect("replaced");
    let listed = call(&state, ana, "template_list", json!({})).await.unwrap();
    assert!(listed.contains("for: the title card alone"), "{listed}");
}

#[sqlx::test]
async fn a_description_too_long_saves_nothing(pool: PgPool) {
    let (_, state) = common::serve_with(pool.clone(), common::files("templates-long")).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let project = episode(&pool, ana).await;
    let arguments = json!({ "project": project, "clips": ["c-title"], "name": "Long",
                            "description": "x".repeat(2001) });
    let refused = call(&state, ana, "template_save", arguments).await;
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.contains("at most 2000") && why.contains("nothing was saved")),
        "{refused:?}"
    );
    let listed = call(&state, ana, "template_list", json!({})).await.unwrap();
    assert!(listed.contains("no templates"), "{listed}");
}
