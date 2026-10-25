//! The bonus on larger recharges (#1537): quoted at its tier, locked on the
//! payment, credited once beside the top-up, and taken back by a withdrawal.

use std::sync::Arc;

use scorsese_server::db::{self, UserId};
use scorsese_server::payments::settle;
use sqlx::postgres::PgPool;

use super::common::request;
use std::net::SocketAddr;

use scorsese_server::payments::Bonus;

use super::{Fake, balance, bought, member, rates, serve_with, top_ups};

/// What R$ 40,00 buys at 4.99, and its 5%.
pub(super) const FORTY: (i64, i64) = (8_016_032, 400_801);
/// What R$ 80,00 buys at 4.99, and its 10%.
pub(super) const EIGHTY: (i64, i64) = (16_032_064, 1_603_206);

/// [`serve`](super::serve), adding the default recharge bonus.
pub(super) async fn serve_bonus(pool: &PgPool, fake: Arc<Fake>, rate: &'static str) -> SocketAddr {
    serve_with(pool, Some(fake), &rates(Some(rate)), Bonus::default()).await
}

/// How many entries of `kind` `user` has.
pub(super) async fn entries(pool: &PgPool, user: UserId, kind: &str) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    sqlx::query_scalar("SELECT count(*) FROM credit_entries WHERE kind = $1")
        .bind(kind)
        .fetch_one(&mut *tx)
        .await
        .expect("the ledger reads")
}

#[sqlx::test]
async fn the_quote_carries_the_bonus_from_each_tiers_first_centavo(pool: PgPool) {
    let address = serve_bonus(&pool, Arc::new(Fake::default()), "4.99").await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    for (centavos, percent, bonus) in [
        (2_000, 0, 0),
        (3_999, 0, 0),
        (4_000, 5, FORTY.1),
        (7_999, 5, 801_503),
        (8_000, 10, EIGHTY.1),
    ] {
        let path = format!("/api/payments/quote?centavos={centavos}");
        let quote = request(address, "GET", &path, &[&bearer], None).await;
        assert_eq!(quote.status, 200, "{}", quote.body);
        let quote = quote.json();
        assert_eq!(quote["bonus_percent"], percent, "{centavos}");
        assert_eq!(quote["bonus_micros"], bonus, "{centavos}");
    }
}

#[sqlx::test]
async fn each_tier_credits_its_bonus_once_however_the_confirmations_race(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve_bonus(&pool, fake.clone(), "4.99").await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let mut expected = 0;
    for (centavos, (credit, bonus)) in [(4_000, FORTY), (8_000, EIGHTY), (2_000, (4_008_016, 0))] {
        let session = bought(address, &bearer, centavos).await;
        let paid = fake.pay(&session);
        let path = format!("/api/payments/{session}/confirm");
        let headers = [bearer.as_str()];
        let (hook, page, also) = tokio::join!(
            settle::settle(&pool, &paid),
            request(address, "POST", &path, &headers, None),
            settle::settle(&pool, &paid),
        );
        hook.unwrap();
        also.unwrap();
        assert_eq!(page.status, 200, "{}", page.body);
        let page = page.json();
        assert_eq!(page["credit_micros"], credit);
        assert_eq!(page["bonus_micros"], bonus);
        expected += credit + bonus;
        assert_eq!(balance(&pool, ana).await, expected, "R$ {centavos}");
    }
    assert_eq!(top_ups(&pool, ana).await, 3);
    assert_eq!(
        entries(&pool, ana, "bonus").await,
        2,
        "R$ 20 has no bonus entry"
    );
}
