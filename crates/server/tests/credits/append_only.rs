//! The ledger is append-only, and the database is what says so.

use scorsese_server::accounts::users;
use scorsese_server::credits::generations::Answer;
use scorsese_server::db;
use sqlx::Executor;
use sqlx::postgres::PgPool;

use super::{account, finish, fund, shot, start};

#[sqlx::test]
async fn a_member_can_neither_update_nor_delete_an_entry(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let members = db::member_pool(&pool).await.unwrap();
    for query in [
        "UPDATE credit_entries SET amount_micros = 999999999",
        "DELETE FROM credit_entries",
    ] {
        let mut tx = db::scoped(&members, ana).await.unwrap();
        let error = sqlx::query(query).execute(&mut *tx).await.unwrap_err();
        assert!(
            error.to_string().contains("permission denied"),
            "{query}: {error}"
        );
    }
}

/// Not even the login role — which bypasses row-level security and holds
/// every grant — can rewrite money that has moved.
#[sqlx::test]
async fn nobody_can_rewrite_the_ledger(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    for query in [
        "UPDATE credit_entries SET amount_micros = 1",
        "DELETE FROM credit_entries",
        "TRUNCATE credit_entries CASCADE",
    ] {
        let mut tx = db::privileged(&pool).await.unwrap();
        let error = tx.execute(query).await.unwrap_err();
        assert!(
            error.to_string().contains("append-only"),
            "{query}: {error}"
        );
    }
}

/// The one way entries leave: the account they belong to is deleted, and the
/// ledger, the generations and everything else go with it.
#[sqlx::test]
async fn deleting_an_account_takes_its_ledger_with_it(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let bia = account(&pool, "bia@example.com").await;
    fund(&pool, ana, 10).await;
    fund(&pool, bia, 10).await;
    let paid = start(&pool, ana, &shot(None)).await.unwrap();
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();

    let storage = std::env::temp_dir().join(format!("scorsese-537-{}", ana.get()));
    users::delete(&pool, &storage, "ana@example.com")
        .await
        .unwrap();

    let mut tx = db::privileged(&pool).await.unwrap();
    let left: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM credit_entries
         UNION ALL SELECT user_id FROM veo_generations",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(left, [bia.get()]);
}

#[sqlx::test]
async fn members_read_the_display_rate_and_cannot_set_it(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = db::member_pool(&pool).await.unwrap();
    let mut tx = db::scoped(&members, ana).await.unwrap();
    let error = sqlx::query("INSERT INTO display_rates (brl_per_usd_e4) VALUES (1)")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("permission denied"), "{error}");

    let mut tx = db::scoped(&members, ana).await.unwrap();
    let rates: i64 = sqlx::query_scalar("SELECT count(*) FROM display_rates")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(rates, 0);
}
