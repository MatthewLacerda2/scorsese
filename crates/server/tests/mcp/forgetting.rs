//! Forgetting a kept voice (#1381): deleted at the vendor and from the
//! user's ledger, so the slot it took is free — by the user, or for them when
//! their account is deleted.

use scorsese_server::generations::Vendors;
use scorsese_server::operator::{self, UserCommand};
use scorsese_server::storage::Storage;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::fund;
use super::vendors::{Mock, serve, settled};
use super::voicing::design_and_keep;
use super::{call, member};

/// The voice `design_and_keep` keeps.
const KEPT: &str = "voice-of-gen-1";

#[sqlx::test]
async fn a_forgotten_voice_is_gone_and_its_slot_free(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    design_and_keep(address, &state, ana, &hers).await;

    let (said, refused) = call(address, &hers, "voice_design", json!({ "forget": KEPT })).await;
    assert!(!refused, "{said}");
    assert!(said.contains("Night editor"), "{said}");
    settled(&state, ana).await;

    assert_eq!(mock.forgotten(), 1);
    let (listed, _) = call(address, &hers, "voice_design", json!({ "list": true })).await;
    assert!(!listed.contains(KEPT), "{listed}");
    let (said, refused) = call(
        address,
        &hers,
        "voice_design",
        json!({ "keep": "gen-1", "name": "Back again" }),
    )
    .await;
    assert!(
        !refused,
        "the design stayed, so it can be kept again: {said}"
    );
}

#[sqlx::test]
async fn only_a_voice_of_ones_own_can_be_forgotten(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (_, his) = member(&pool, "bob@example.com").await;
    fund(&pool, ana, 10).await;
    design_and_keep(address, &state, ana, &hers).await;

    let (said, refused) = call(address, &his, "voice_design", json!({ "forget": KEPT })).await;
    assert!(refused, "{said}");
    assert!(said.contains("nothing was deleted"), "{said}");
    assert_eq!(mock.forgotten(), 0, "the vendor was never asked");
}

/// Deleting the account deletes its voices first; one the vendor will not
/// delete is said, for the operator, and does not stop the deletion.
#[sqlx::test]
async fn deleting_an_account_forgets_its_voices(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (bob, his) = member(&pool, "bob@example.com").await;
    fund(&pool, ana, 10).await;
    fund(&pool, bob, 10).await;
    design_and_keep(address, &state, ana, &hers).await;
    design_and_keep(address, &state, bob, &his).await;

    let said = delete(&pool, &mock, "ana@example.com").await;
    assert_eq!(mock.forgotten(), 1);
    assert!(!said.contains("could not be deleted"), "{said}");

    let refusing = Mock {
        refuse_forget: Some("the vendor is down".to_owned()),
        ..mock.clone()
    };
    let said = delete(&pool, &refusing, "bob@example.com").await;
    assert!(said.contains("deleted account"), "{said}");
    assert!(said.contains(KEPT), "{said}");
    assert!(said.contains("the vendor is down"), "{said}");
}

/// The operator's `user delete --yes`, reaching `vendors`.
async fn delete(pool: &PgPool, vendors: &dyn Vendors, email: &str) -> String {
    let temp = std::env::temp_dir();
    let files = Storage::new(
        temp.join("scorsese-forgetting-kept"),
        temp.join("scorsese-forgetting-cache"),
    );
    let command = UserCommand::Delete {
        email: email.to_owned(),
        yes: true,
    };
    operator::user(pool, &files, vendors, command)
        .await
        .expect("the account is deleted")
}
