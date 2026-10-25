//! Veo is offered only with paid credit (#1530): on free usage the generate
//! tool says nothing of video, and the turn is told why.

use scorsese_server::credits::allowance::{self, Allowance};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, exchange, member, project};

/// The first request's generate description, and whether its messages say
/// that Veo is not available.
async fn offered(pool: &PgPool, dollars: i64) -> (String, bool) {
    let script = Script::new(vec![answers("Here is an intro.")]);
    let (address, _) = super::scripted(pool, &script).await;
    allowance::set(pool, Allowance::DEFAULT).await.unwrap();
    let (ana, who) = member(pool, "ana@example.com", dollars).await;
    let id = project(pool, ana, json!([])).await;
    let detail = exchange(address, &who, id, "make an intro").await;
    assert_eq!(detail["turn"]["state"], "answered", "{detail}");
    let sent = &script.requests()[0];
    let generate = sent
        .tools
        .iter()
        .find(|tool| tool.name == "generate")
        .expect("generate is offered either way");
    let told = script
        .messages(0)
        .iter()
        .any(|message| message.contains("Veo is not available because the user is on free usage"));
    (generate.description.clone(), told)
}

#[sqlx::test]
async fn on_free_usage_the_assistant_is_not_offered_veo_and_is_told_why(pool: PgPool) {
    let (description, told) = offered(&pool, 0).await;
    assert!(!description.contains("Veo"), "{description}");
    assert!(!description.contains("generated_video"), "{description}");
    assert!(told);
}

#[sqlx::test]
async fn with_paid_credit_the_assistant_is_offered_veo(pool: PgPool) {
    let (description, told) = offered(&pool, 10).await;
    assert!(description.contains("Veo shots"), "{description}");
    assert!(!told);
}
