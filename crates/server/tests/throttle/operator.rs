//! What the operator sees of the brake, and how they lift it.

use scorsese_server::accounts::throttle::counter::LIMIT;
use scorsese_server::accounts::throttle::{self, Key};
use scorsese_server::operator::{self, UserCommand};
use scorsese_server::storage::Storage;
use sqlx::postgres::PgPool;

async fn run(pool: &PgPool, command: UserCommand) -> String {
    let temp = std::env::temp_dir();
    let files = Storage::new(
        temp.join("scorsese-throttle-kept"),
        temp.join("scorsese-throttle-cache"),
    );
    operator::user(pool, &files, command)
        .await
        .expect("the command succeeds")
}

/// Lock `key` out, as `LIMIT` attempts and one more would.
async fn lock(pool: &PgPool, key: &Key) {
    for _ in 0..=LIMIT {
        let _ = throttle::admit(pool, std::slice::from_ref(key))
            .await
            .expect("the throttle table answers");
    }
}

#[sqlx::test]
async fn locks_are_listed_and_lifted_by_email_or_address(pool: PgPool) {
    assert_eq!(
        run(&pool, UserCommand::Locks).await,
        "nothing is locked out"
    );
    lock(&pool, &Key::email("ana@example.com")).await;
    lock(&pool, &Key::Address("2001:db8:1:2::/64".into())).await;
    throttle::admit(&pool, &[Key::email("bia@example.com")])
        .await
        .unwrap()
        .unwrap();

    let listing = run(&pool, UserCommand::Locks).await;
    let lines: Vec<&str> = listing.lines().collect();
    assert_eq!(lines.len(), 3, "{listing}");
    assert!(
        lines[0].starts_with("address\t2001:db8:1:2::/64\tlocked for 15 more min"),
        "{listing}"
    );
    assert!(
        lines[1].starts_with("email\tana@example.com\tlocked for 15 more min"),
        "{listing}"
    );
    assert_eq!(
        lines[2],
        "email\tbia@example.com\t1 of 10 attempts used in the last 15 min"
    );

    let who = "2001:db8:1:2::77".to_owned();
    let lifted = run(&pool, UserCommand::Unlock { who: who.clone() }).await;
    assert_eq!(lifted, "address 2001:db8:1:2::/64 may log in again");
    let again = run(&pool, UserCommand::Unlock { who }).await;
    assert_eq!(again, "address 2001:db8:1:2::/64 was not being braked");
    let email = run(
        &pool,
        UserCommand::Unlock {
            who: "Ana@Example.com".into(),
        },
    )
    .await;
    assert_eq!(email, "email ana@example.com may log in again");
    assert_eq!(run(&pool, UserCommand::Locks).await.lines().count(), 1);
}

#[sqlx::test]
async fn a_password_reset_lifts_the_lock_on_that_email(pool: PgPool) {
    let email = "ana@example.com".to_owned();
    run(
        &pool,
        UserCommand::Create {
            email: email.clone(),
        },
    )
    .await;
    lock(&pool, &Key::email(&email)).await;
    run(&pool, UserCommand::ResetPassword { email }).await;
    assert_eq!(
        run(&pool, UserCommand::Locks).await,
        "nothing is locked out"
    );
}
