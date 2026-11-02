//! A turn refused because every account together has spent the day's free
//! ceiling (#1658) says so, not that the user has no credit.

use scorsese_server::credits::allowance::{self, Allowance};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, member, project, send};

#[sqlx::test]
async fn a_turn_past_everyones_ceiling_says_free_credit_comes_back_tomorrow(pool: PgPool) {
    let nothing_for_everyone = Allowance {
        total_daily_micros: 0,
        ..Allowance::DEFAULT
    };
    allowance::set(&pool, nothing_for_everyone).await.unwrap();
    let script = Script::new(vec![answers("never")]);
    let (address, _) = super::scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 0).await;
    let id = project(&pool, ana, json!([])).await;
    let refused = send(address, &who, id, "make an intro").await;
    assert_eq!(refused.status, 402, "{}", refused.body);
    assert!(
        refused.body.contains("free credit for everyone is used up"),
        "{}",
        refused.body
    );
    assert!(refused.body.contains("tomorrow"), "{}", refused.body);
    assert!(script.requests().is_empty());
}
