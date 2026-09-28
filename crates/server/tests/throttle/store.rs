//! The table the counters live in: all-or-nothing counting, forgetting, and
//! nobody but the login and the operator reading it.

use scorsese_server::accounts::throttle::counter::LIMIT;
use scorsese_server::accounts::throttle::{self, Key};
use scorsese_server::accounts::users;
use scorsese_server::db;
use sqlx::postgres::PgPool;

fn keys() -> (Key, Key) {
    (
        Key::email("ana@example.com"),
        Key::Address("203.0.113.9".into()),
    )
}

#[sqlx::test]
async fn an_attempt_one_counter_refuses_is_not_counted_by_the_other(pool: PgPool) {
    let (email, address) = keys();
    for _ in 0..LIMIT {
        throttle::admit(&pool, std::slice::from_ref(&email))
            .await
            .unwrap()
            .unwrap();
    }
    let both = [email.clone(), address.clone()];
    let wait = throttle::admit(&pool, &both).await.unwrap().unwrap_err();
    assert_eq!(wait.as_secs(), 15 * 60);
    assert!(throttle::admit(&pool, &both).await.unwrap().is_err());

    let locks = throttle::locks(&pool).await.unwrap();
    assert_eq!(locks.len(), 1, "{locks:?}");
    assert_eq!((locks[0].key.clone(), locks[0].strikes), (email, 1));
}

#[sqlx::test]
async fn a_success_clears_the_email_and_gives_the_address_its_attempt_back(pool: PgPool) {
    let (email, address) = keys();
    let both = [email.clone(), address.clone()];
    for _ in 0..3 {
        throttle::admit(&pool, &both).await.unwrap().unwrap();
    }
    throttle::succeeded(&pool, &email, Some(&address))
        .await
        .unwrap();
    let locks = throttle::locks(&pool).await.unwrap();
    assert_eq!(locks.len(), 1, "{locks:?}");
    assert_eq!((locks[0].key.clone(), locks[0].attempts), (address, 2));
}

#[sqlx::test]
async fn a_row_quiet_for_a_day_is_forgotten(pool: PgPool) {
    let (email, address) = keys();
    throttle::admit(&pool, &[email]).await.unwrap().unwrap();
    sqlx::query("UPDATE login_throttle SET touched = touched - 90000, window_start = 0")
        .execute(&pool)
        .await
        .unwrap();
    throttle::admit(&pool, &[address]).await.unwrap().unwrap();
    let kinds: Vec<String> = sqlx::query_scalar("SELECT kind FROM login_throttle")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(kinds, ["address"]);
}

#[sqlx::test]
async fn a_member_cannot_read_whose_email_is_being_guessed(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com", "password one")
        .await
        .unwrap();
    throttle::admit(&pool, &[keys().0]).await.unwrap().unwrap();
    let members = db::member_pool(&pool).await.unwrap();
    let mut tx = db::scoped(&members, ana).await.unwrap();
    let error = sqlx::query("SELECT count(*) FROM login_throttle")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("permission denied"), "{error}");
}
