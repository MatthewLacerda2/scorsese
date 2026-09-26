//! Stored projects below HTTP: which files each one uses, racing writers, and
//! the migration the server runs over every document when it starts.

mod common;

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, SCHEMA_VERSION};
use scorsese_server::ServerError;
use scorsese_server::accounts::users;
use scorsese_server::db::{self, UserId};
use scorsese_server::projects::{self, ProjectError, media};
use sqlx::postgres::PgPool;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A project whose assets carry these hashes, one asset each.
fn using(hashes: &[&str]) -> Project {
    let mut project = Project::new("p", Fps::default());
    for (n, hash) in hashes.iter().enumerate() {
        let path = media::library_path(hash, Some("mp4"));
        let mut asset = Asset::imported(AssetId::new(format!("clip{n}")), AssetKind::Video, path);
        asset.sha256 = Some((*hash).to_owned());
        project.assets.push(asset);
    }
    project
}

async fn account(pool: &PgPool, email: &str) -> UserId {
    users::create(pool, email, "password one")
        .await
        .expect("the account is created")
}

async fn files_of(pool: &PgPool, project: i64) -> Vec<String> {
    sqlx::query_scalar("SELECT sha256 FROM project_assets WHERE project_id = $1 ORDER BY sha256")
        .bind(project)
        .fetch_all(pool)
        .await
        .expect("project_assets reads")
}

#[sqlx::test]
async fn which_files_a_project_uses_follows_every_write(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = db::member_pool(&pool).await.unwrap();

    // A duplicate is one file; a malformed hash names none.
    let mut project = using(&[A, A, B, "NOT-A-HASH"]);
    let id = projects::create(&members, ana, &project).await.unwrap().id;
    assert_eq!(files_of(&pool, id).await, [A, B]);

    project
        .assets
        .retain(|asset| asset.sha256.as_deref() != Some(A));
    projects::save(&members, ana, id, 1, &project)
        .await
        .unwrap();
    assert_eq!(files_of(&pool, id).await, [B]);

    projects::edit(&members, ana, id, |project| project.assets.clear())
        .await
        .unwrap();
    assert!(files_of(&pool, id).await.is_empty());

    projects::save(&members, ana, id, 3, &using(&[A]))
        .await
        .unwrap();
    assert!(projects::delete(&members, ana, id).await.unwrap());
    assert!(files_of(&pool, id).await.is_empty(), "deleting cascades");
}

#[sqlx::test]
async fn of_two_writers_racing_from_one_revision_exactly_one_lands(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = db::member_pool(&pool).await.unwrap();
    let id = projects::create(&members, ana, &using(&[]))
        .await
        .unwrap()
        .id;

    let (mine, theirs) = (using(&[A]), using(&[B]));
    let (first, second) = tokio::join!(
        projects::save(&members, ana, id, 1, &mine),
        projects::save(&members, ana, id, 1, &theirs),
    );
    let landed = [&first, &second]
        .iter()
        .filter(|outcome| outcome.is_ok())
        .count();
    assert_eq!(landed, 1, "{first:?} {second:?}");
    let refused = if first.is_err() { first } else { second };
    assert!(
        matches!(refused, Err(ProjectError::Conflict { current: 2 })),
        "{refused:?}"
    );
}

#[sqlx::test]
async fn documents_already_current_are_not_touched_on_start(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = db::member_pool(&pool).await.unwrap();
    let id = projects::create(&members, ana, &using(&[A]))
        .await
        .unwrap()
        .id;

    assert_eq!(projects::migrate_stored(&pool).await.unwrap(), 0);
    let opened = projects::open(&members, ana, id).await.unwrap();
    assert_eq!(opened.summary.revision, 1);
}

#[sqlx::test(migrations = false)]
async fn a_document_this_build_cannot_carry_forward_stops_the_start(pool: PgPool) {
    db::migrate(&pool).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    let newer = format!(
        r#"{{ "schema_version": {}, "name": "from the future" }}"#,
        SCHEMA_VERSION + 1
    );
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (user_id, document) VALUES ($1, $2::jsonb) RETURNING id",
    )
    .bind(ana.get())
    .bind(newer)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (listener, _) = common::listener().await;
    let outcome = scorsese_server::start(pool.clone(), listener, std::future::pending()).await;
    assert!(
        matches!(
            &outcome,
            Err(ServerError::Projects(ProjectError::Migrate { id: named, .. })) if *named == id
        ),
        "{outcome:?}"
    );
}
