//! Buying credits (`src/payments`, #548): a quote, a Checkout Session, and
//! crediting it exactly once from the webhook or the return page. Stripe and
//! the rate sources are stand-ins here ([`Fake`], [`Fixed`]); nothing reaches
//! a network.

use crate::common;

mod bonus;
mod confirm;
mod fake;
mod open;
mod prices;
mod rate;
mod start;
mod webhook;
mod withdraw;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use scorsese_providers::credentials::Secret;
use scorsese_providers::fx::{FxError, Latest, Rate, Rates, Source};
use scorsese_server::accounts::{tokens, users};
use scorsese_server::credits::ledger;
use scorsese_server::db::{self, UserId};
use scorsese_server::http;
use scorsese_server::payments::{Bonus, Payments};
use sqlx::postgres::PgPool;

pub(crate) use fake::Fake;

/// The webhook's signing secret in every test.
const SECRET: &str = "whsec_test";

/// A rate source answering one figure, or failing; counting its calls.
struct Fixed(Option<&'static str>, AtomicUsize);

impl Rates for Fixed {
    fn usd_brl(&self) -> Result<Rate, FxError> {
        self.1.fetch_add(1, Ordering::SeqCst);
        self.0
            .and_then(|text| Rate::parse(text, Source::Awesome))
            .ok_or_else(|| FxError {
                awesome: "429".into(),
                ptax: "unreachable".into(),
            })
    }
}

/// A served server selling through `fake` at `rate` (`None`: no rate can be
/// read), or — with no `fake` — one with no Stripe configured.
async fn serve(pool: &PgPool, fake: Option<Arc<Fake>>, rate: Option<&'static str>) -> SocketAddr {
    serve_kept(pool, fake, &rates(rate)).await
}

/// The rate a server keeps, read from a source answering `rate` (`None`:
/// failing), with nothing read yet.
fn rates(rate: Option<&'static str>) -> Arc<Latest> {
    Arc::new(Latest::new(Arc::new(Fixed(rate, AtomicUsize::new(0)))))
}

/// [`serve`], keeping its rate in `latest` — for a test that ages it.
async fn serve_kept(pool: &PgPool, fake: Option<Arc<Fake>>, latest: &Arc<Latest>) -> SocketAddr {
    serve_with(pool, fake, latest, Bonus::none()).await
}

/// A served server selling through `fake` at the rate kept in `latest`,
/// adding `bonus`.
async fn serve_with(
    pool: &PgPool,
    fake: Option<Arc<Fake>>,
    latest: &Arc<Latest>,
    bonus: Bonus,
) -> SocketAddr {
    let (listener, address) = common::listener().await;
    db::migrate(pool).await.expect("the migrations apply");
    let members = db::member_pool(pool)
        .await
        .expect("the member pool connects");
    let payments = fake.map(|fake| Payments {
        checkout: fake,
        rates: latest.clone(),
        webhook_secret: Secret::new(SECRET),
        origin: "https://scorsese.example".into(),
        bonus,
    });
    let state =
        http::AppState::new(members.clone(), common::files("payments")).with_payments(payments);
    let server = tokio::spawn(http::serve(
        listener,
        http::router(state),
        std::future::pending(),
    ));
    common::release_at_teardown(pool, &members, [server.abort_handle()]);
    address
}

/// A new account for `email`, and the bearer header its token sends.
async fn member(pool: &PgPool, email: &str) -> (UserId, String) {
    let user = users::create(pool, email)
        .await
        .expect("a fresh database takes an account");
    let issued = tokens::issue(pool, user, "test").await;
    let token = issued.expect("a new account takes a token").token;
    (user, format!("Authorization: Bearer {token}"))
}

/// `user`'s balance, in micro-dollars.
async fn balance(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    ledger::balance(&mut tx).await.expect("the balance reads")
}

/// How many top-ups `user` has.
async fn top_ups(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    sqlx::query_scalar("SELECT count(*) FROM credit_entries WHERE kind = 'top_up'")
        .fetch_one(&mut *tx)
        .await
        .expect("the ledger reads")
}

/// Start a purchase of `centavos` as `bearer`; the session id.
async fn bought(address: SocketAddr, bearer: &str, centavos: i64) -> String {
    let body = serde_json::json!({ "centavos": centavos });
    let started = common::request(address, "POST", "/api/payments", &[bearer], Some(&body)).await;
    assert_eq!(started.status, 200, "{}", started.body);
    let session = started.json()["session"].as_str().map(str::to_owned);
    session.expect("a started purchase names its session")
}

/// What R$ 50,00 buys at 4.99: $10.020040.
const FIFTY_AT_4_99: i64 = 10_020_040;
