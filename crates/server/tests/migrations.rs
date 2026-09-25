//! The migration harness: every file in `migrations/` is one sqlx will run,
//! and running them is safe to repeat on every start.
//!
//! The naming test needs no database. The other one does, and fails without
//! `DATABASE_URL` rather than skipping — see the crate doc.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use scorsese_server::db;
use sqlx::postgres::PgPool;

/// The one file in `migrations/` that is not a migration.
const README: &str = "README.md";

/// The version a file named `NNNN_what_it_does.sql` declares, or why it is
/// not named that way.
fn version(name: &str) -> Result<i64, String> {
    let (digits, rest) = name
        .split_once('_')
        .ok_or_else(|| format!("{name}: no `_` after the version"))?;
    let description = rest
        .strip_suffix(".sql")
        .ok_or_else(|| format!("{name}: does not end in .sql"))?;
    let lower_snake = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_';
    if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("{name}: the version is not four digits"));
    }
    if description.is_empty() || !description.chars().all(lower_snake) {
        return Err(format!("{name}: the description is not lower_snake_case"));
    }
    digits.parse().map_err(|_| format!("{name}: unparseable"))
}

#[test]
fn every_file_is_a_migration_numbered_one_after_the_last() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut versions = BTreeSet::new();
    for entry in fs::read_dir(&directory).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name == README {
            continue;
        }
        let version = version(&name).unwrap_or_else(|why| panic!("{why}"));
        assert!(
            versions.insert(version),
            "{name}: version {version} is taken"
        );
    }
    let expected: BTreeSet<i64> = (1..=versions.len() as i64).collect();
    assert_eq!(
        versions, expected,
        "versions must run 1, 2, 3… with no gaps"
    );

    // And sqlx agrees: each of those files is embedded in the binary.
    let embedded: BTreeSet<i64> = db::MIGRATOR.iter().map(|m| m.version).collect();
    assert_eq!(embedded, versions, "a file sqlx did not pick up");
}

#[test]
fn a_misnamed_file_is_caught() {
    for bad in [
        "0002-users.sql",
        "2_users.sql",
        "0002_users.SQL",
        "0002_Users.sql",
        "0002_.sql",
    ] {
        assert!(version(bad).is_err(), "{bad} was accepted");
    }
    assert_eq!(version("0012_add_users.sql"), Ok(12));
}

#[sqlx::test(migrations = false)]
async fn migrating_a_fresh_database_twice_is_the_same_as_once(pool: PgPool) {
    db::migrate(&pool).await.unwrap();
    db::migrate(&pool).await.unwrap();
    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(applied, db::MIGRATOR.iter().count() as i64);
}
