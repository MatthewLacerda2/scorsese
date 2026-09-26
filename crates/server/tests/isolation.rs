//! Per-user isolation (`src/db/scope.rs`): every table follows the pattern,
//! and a query that forgot its scope is refused rather than answered.

use scorsese_server::accounts::{tokens, users};
use scorsese_server::db;
use sqlx::Executor;
use sqlx::postgres::PgPool;

/// Tables that are not a user's data. Adding one here is a decision a
/// reviewer should see argued in the same pull request.
///
/// `display_rates` (#537): the reais-per-dollar rate balances are shown at is
/// one fact about the world, the same for every user, set by the operator.
/// Members may read it and write nothing — the migration revokes the rest,
/// and `tests/credits/append_only.rs` holds that.
const NOT_PER_USER: &[&str] = &["_sqlx_migrations", "display_rates"];

#[sqlx::test]
async fn every_table_is_owned_cascaded_and_policed(pool: PgPool) {
    let tables: Vec<(String, bool, bool, bool)> = sqlx::query_as(
        "SELECT c.relname::text,
            c.relrowsecurity,
            EXISTS (SELECT 1 FROM pg_policies p
                    WHERE p.schemaname = 'public' AND p.tablename = c.relname
                      AND p.qual LIKE '%member_id()%'),
            EXISTS (SELECT 1 FROM pg_constraint k
                    JOIN pg_attribute a ON a.attrelid = k.conrelid AND a.attnum = ANY (k.conkey)
                    WHERE k.conrelid = c.oid AND k.contype = 'f'
                      AND k.confrelid = 'users'::regclass AND k.confdeltype = 'c'
                      AND a.attname = 'user_id' AND a.attnotnull)
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = 'public' AND c.relkind IN ('r', 'p')",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        tables.iter().any(|(name, ..)| name == "users"),
        "{tables:?}"
    );

    for (table, secured, policed, owned) in tables {
        if NOT_PER_USER.contains(&table.as_str()) {
            continue;
        }
        let why = "see crates/server/src/db/scope.rs for the pattern";
        assert!(secured, "{table}: row-level security is not enabled; {why}");
        assert!(
            policed,
            "{table}: no policy compares with member_id(); {why}"
        );
        assert!(
            owned || table == "users",
            "{table}: no `user_id NOT NULL REFERENCES users ON DELETE CASCADE`; {why}"
        );
    }
}

#[sqlx::test]
async fn a_query_outside_a_scope_is_refused_even_on_an_empty_table(pool: PgPool) {
    let members = db::member_pool(&pool).await.unwrap();
    for query in [
        "SELECT count(*) FROM users",
        "SELECT count(*) FROM sessions",
        "SELECT count(*) FROM api_tokens",
        "SELECT count(*) FROM jobs",
        "SELECT count(*) FROM projects",
        "SELECT count(*) FROM project_assets",
        "SELECT count(*) FROM credit_entries",
        "SELECT count(*) FROM veo_generations",
        "SELECT count(*) FROM speech_generations",
        "SELECT count(*) FROM renders",
        "SELECT count(*) FROM display_rates",
        "SELECT count(*) FROM library_items",
        "SELECT count(*) FROM uploads",
    ] {
        let error = sqlx::query(query).execute(&members).await.unwrap_err();
        assert!(
            error.to_string().contains("permission denied"),
            "{query}: {error}"
        );
    }
}

#[sqlx::test]
async fn the_member_role_without_a_user_says_so(pool: PgPool) {
    users::create(&pool, "a@example.com", "password one")
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    tx.execute("SET LOCAL ROLE scorsese_member").await.unwrap();
    let error = sqlx::query("SELECT id FROM users")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("outside a user scope"),
        "{error}"
    );
}

#[sqlx::test]
async fn a_scope_sees_and_writes_only_its_own_rows(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com", "password one")
        .await
        .unwrap();
    let bia = users::create(&pool, "bia@example.com", "password two")
        .await
        .unwrap();
    tokens::issue(&pool, ana, "ana's").await.unwrap();
    tokens::issue(&pool, bia, "bia's").await.unwrap();

    let members = db::member_pool(&pool).await.unwrap();
    let mut tx = db::scoped(&members, ana).await.unwrap();
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM api_tokens")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(names, ["ana's"]);
    let emails: Vec<String> = sqlx::query_scalar("SELECT email FROM users")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(emails, ["ana@example.com"]);

    // Writing a row that names somebody else as its owner is refused.
    let error =
        sqlx::query("INSERT INTO api_tokens (user_id, name, token_hash) VALUES ($1, 'x', 'x')")
            .bind(bia.get())
            .execute(&mut *tx)
            .await
            .unwrap_err();
    assert!(error.to_string().contains("row-level security"), "{error}");
}

#[sqlx::test]
async fn a_row_cannot_claim_another_users_project(pool: PgPool) {
    // The policy checks the owner column; the composite foreign key is what
    // stops a row that names *itself* as owner from pointing at somebody
    // else's project.
    let ana = users::create(&pool, "ana@example.com", "password one")
        .await
        .unwrap();
    let bia = users::create(&pool, "bia@example.com", "password two")
        .await
        .unwrap();
    let anas: i64 = sqlx::query_scalar(
        "INSERT INTO projects (user_id, document) VALUES ($1, '{\"name\": \"a\"}') RETURNING id",
    )
    .bind(ana.get())
    .fetch_one(&pool)
    .await
    .unwrap();

    let members = db::member_pool(&pool).await.unwrap();
    let mut tx = db::scoped(&members, bia).await.unwrap();
    let error = sqlx::query(
        "INSERT INTO project_assets (project_id, user_id, sha256) VALUES ($1, member_id(), $2)",
    )
    .bind(anas)
    .bind("a".repeat(64))
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert!(error.to_string().contains("foreign key"), "{error}");
}
