//! `POST /api/login` braking, over HTTP, and which address it believes.

use scorsese_server::accounts::throttle::counter::LIMIT;
use scorsese_server::accounts::users;
use scorsese_server::http::client::Clients;
use sqlx::postgres::PgPool;

use super::{login, serve};

/// The header line naming `address` as the client, as the deploy's nginx
/// would write it.
fn from(address: &str) -> String {
    format!("X-Scorsese-Client: {address}")
}

#[sqlx::test]
async fn a_real_email_and_an_unknown_one_are_braked_alike(pool: PgPool) {
    let address = serve(pool.clone(), Clients::Proxy).await;
    users::create(&pool, "ana@example.com", "correct horse")
        .await
        .unwrap();

    let mut refusals = Vec::new();
    for email in ["ana@example.com", "nobody@example.com"] {
        // Each guess from an address of its own: only the email is counted
        // up to its limit.
        for n in 0..LIMIT {
            let origin = from(&format!("198.51.100.{n}"));
            let wrong = login(address, email, "wrong horse", &[origin.as_str()]).await;
            assert_eq!(wrong.status, 401, "{email} guess {n}");
        }
        let origin = from("192.0.2.1");
        refusals.push(login(address, email, "correct horse", &[origin.as_str()]).await);
    }
    let (ana, nobody) = (&refusals[0], &refusals[1]);
    assert_eq!((ana.status, nobody.status), (429, 429));
    assert_eq!(ana.body, nobody.body);
    assert_eq!(
        ana.json()["error"],
        "too many login attempts; try again in 15 minutes"
    );
    assert_eq!(ana.header("retry-after"), Some("900"));
    assert!(ana.header("set-cookie").is_none());
}

#[sqlx::test]
async fn one_address_is_braked_across_every_email_it_tries(pool: PgPool) {
    let address = serve(pool, Clients::Proxy).await;
    let origin = from("203.0.113.9");
    for n in 0..LIMIT {
        let email = format!("guess{n}@example.com");
        let wrong = login(address, &email, "password", &[origin.as_str()]).await;
        assert_eq!(wrong.status, 401);
    }
    let next = login(address, "fresh@example.com", "password", &[origin.as_str()]).await;
    assert_eq!(next.status, 429);
    let elsewhere = from("203.0.113.10");
    let other = login(
        address,
        "fresh@example.com",
        "password",
        &[elsewhere.as_str()],
    )
    .await;
    assert_eq!(other.status, 401, "another address is not held back");
}

#[sqlx::test]
async fn a_client_cannot_name_its_own_address_to_a_server_reached_directly(pool: PgPool) {
    let address = serve(pool, Clients::Peer).await;
    for n in 0..LIMIT {
        let forged = from(&format!("198.51.100.{n}"));
        let email = format!("guess{n}@example.com");
        let wrong = login(address, &email, "password", &[forged.as_str()]).await;
        assert_eq!(wrong.status, 401);
    }
    let forged = from("192.0.2.200");
    let next = login(address, "fresh@example.com", "password", &[forged.as_str()]).await;
    assert_eq!(
        next.status, 429,
        "every forged address was counted as the peer"
    );
}

#[sqlx::test]
async fn logging_in_clears_the_count_of_ones_own_email(pool: PgPool) {
    let address = serve(pool.clone(), Clients::Proxy).await;
    users::create(&pool, "ana@example.com", "correct horse")
        .await
        .unwrap();
    for round in 0..2 {
        for n in 0..LIMIT - 1 {
            let origin = from(&format!("198.51.{round}.{n}"));
            let wrong = login(address, "ana@example.com", "typo", &[origin.as_str()]).await;
            assert_eq!(wrong.status, 401, "round {round}, typo {n}");
        }
        let origin = from("192.0.2.1");
        let right = login(
            address,
            "ana@example.com",
            "correct horse",
            &[origin.as_str()],
        )
        .await;
        assert_eq!(right.status, 200, "round {round}: {}", right.body);
    }
}
