//! A picker answered with nothing picked, or in words instead, resumes the
//! turn with what the person said, and imports nothing.

use super::*;

#[sqlx::test]
async fn none_picked_or_words_instead_resume_with_what_was_said(pool: PgPool) {
    let Picking {
        script,
        fake,
        address,
        who,
        turn,
        paused,
        ..
    } = picking(&pool, &[1, 2], "Searching for darker ones.").await;
    assert_eq!(paused["turn"]["state"], "asking", "{paused}");
    let stranger = json!({ "picked": ["pixabay-image-3"] });
    let refused = answer(address, &who, turn, &stranger).await;
    assert_eq!(
        refused.status, 400,
        "image 3 was not shown: {}",
        refused.body
    );
    let nothing = answer(address, &who, turn, &json!({})).await;
    assert_eq!(nothing.status, 400, "{}", nothing.body);

    let none = json!({ "picked": [], "answer": "something darker" });
    assert_eq!(answer(address, &who, turn, &none).await.status, 202);
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["state"], "answered", "{done}");
    assert_eq!(done["turn"]["questions"][0]["picked"], json!([]));
    assert_eq!(done["turn"]["questions"][0]["answer"], "something darker");
    let told = told(&script, 2);
    assert!(told.contains("picked none of these"), "{told}");
    assert!(told.contains("They wrote: something darker"), "{told}");
    assert!(fake.imported().is_empty(), "nothing was imported");
    let tools: Vec<&Value> = done["tools"].as_array().unwrap().iter().collect();
    assert_eq!(tools.len(), 1, "only the search: {done}");
}

#[sqlx::test]
async fn a_typed_message_answers_a_picker(pool: PgPool) {
    let Picking {
        script,
        address,
        who,
        project: id,
        turn,
        ..
    } = picking(&pool, &[1, 2], "Looking for footage.").await;
    let typed = send(address, &who, id, "none, use footage instead").await;
    assert_eq!(typed.status, 202, "{}", typed.body);
    assert_eq!(
        typed.json()["id"],
        turn,
        "a message answers, it starts nothing"
    );
    let done = finished(address, &who, turn).await;
    assert_eq!(done["turn"]["answer"], "Looking for footage.", "{done}");
    assert!(done["turn"]["questions"][0].get("picked").is_none());
    let told = told(&script, 2);
    assert!(told.contains("answered in words instead"), "{told}");
    assert!(told.contains("none, use footage instead"), "{told}");
}
