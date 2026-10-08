//! A picker that cannot be shown is refused, and the turn carries on; it is
//! never a tool anybody else is offered.

use super::*;

#[sqlx::test]
async fn results_no_search_of_the_turn_showed_are_refused(pool: PgPool) {
    let script = Script::new(vec![
        calls("pick_stock", picker(&[1, 2])),
        answers("I will search first."),
    ]);
    let fake = Fake::new();
    let (address, _) = stocked(&pool, &script, &fake).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let sent = send(address, &who, id, "open on a sunrise").await;
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(done["turn"]["questions"], json!([]), "nothing paused");
    let result = script.parsed(1).last().unwrap()["content"][0].clone();
    assert_eq!(result["is_error"], true, "{result}");
    let said = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("image 1 is not among the results"), "{said}");
}

#[sqlx::test]
async fn a_picker_not_alone_or_badly_formed_is_refused(pool: PgPool) {
    let script = Script::new(vec![
        calls(
            "pick_stock",
            json!({ "question": "Which?", "candidates": [] }),
        ),
        answers("Fine."),
    ]);
    let fake = Fake::new();
    let (address, _) = stocked(&pool, &script, &fake).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let sent = send(address, &who, id, "open on a sunrise").await;
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    assert_eq!(
        finished(address, &who, turn).await["turn"]["state"],
        "answered"
    );
    let result = script.parsed(1).last().unwrap()["content"][0].clone();
    let said = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("`candidates` must be 2 to 8"), "{said}");
}

#[sqlx::test]
async fn web_mcp_never_lists_the_picker(pool: PgPool) {
    let script = Script::new(Vec::new());
    let (_, state) = stocked(&pool, &script, &Fake::new()).await;
    let listed: Vec<String> = state
        .tools
        .listing()
        .iter()
        .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
        .collect();
    assert!(
        listed.iter().any(|name| name == "stock_search"),
        "{listed:?}"
    );
    assert!(
        !listed.iter().any(|name| name == "pick_stock"),
        "{listed:?}"
    );
}
