//! The operator's one stock key is shared by every member (#1652): one import
//! asks the library for at most 20 ids, and one member reaches it at most 30
//! times a minute, while another member's request still goes out.

use scorsese_server::db::UserId;
use scorsese_server::tools::Client;

use super::*;

/// What `user`'s `stock_import` of images `ids` into `project` said, refusal
/// or not.
async fn import(state: &AppState, user: UserId, project: i64, ids: &[u64]) -> String {
    let arguments = json!({ "project": project, "id": ids, "kind": "image" });
    let client = Client::External { token: None };
    match state
        .tools
        .call(user, client, "stock_import", &arguments)
        .await
    {
        Ok(reply) => reply.parts.iter().map(|part| part.text.clone()).collect(),
        Err(refused) => refused,
    }
}

#[sqlx::test]
async fn an_import_names_at_most_twenty_ids(pool: PgPool) {
    let (script, fake) = (Script::new(Vec::new()), Fake::new());
    let (_, state) = stocked(&pool, &script, &fake).await;
    let (ana, _) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;

    let ids: Vec<u64> = (100..121).collect();
    let said = import(&state, ana, id, &ids).await;
    assert!(said.contains("at most 20"), "{said}");
    assert_eq!(fake.asked(), 0, "nothing was asked of the library");
}

#[sqlx::test]
async fn one_member_reaches_the_library_thirty_times_a_minute(pool: PgPool) {
    let (script, fake) = (Script::new(Vec::new()), Fake::new());
    let (_, state) = stocked(&pool, &script, &fake).await;
    let (ana, _) = member(&pool, "ana@example.com", 10).await;
    let (bia, _) = member(&pool, "bia@example.com", 10).await;
    let (hers, theirs) = (
        project(&pool, ana, json!([])).await,
        project(&pool, bia, json!([])).await,
    );

    // Ids the fake has none of: each is one lookup, and nothing downloads.
    let first: Vec<u64> = (100..120).collect();
    let second: Vec<u64> = (120..130).collect();
    import(&state, ana, hers, &first).await;
    import(&state, ana, hers, &second).await;
    assert_eq!(fake.asked(), 30);

    let said = import(&state, ana, hers, &[130]).await;
    assert_eq!(fake.asked(), 30, "the 31st was never asked: {said}");
    assert!(said.contains("try again in"), "{said}");

    let said = import(&state, bia, theirs, &[140]).await;
    assert_eq!(fake.asked(), 31, "another member's still goes out: {said}");
    assert!(said.contains("Fake has no image 140"), "{said}");
}
