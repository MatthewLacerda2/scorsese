//! A stored project's recipes and script (#560): kept beside the document
//! under its revision, laid out with it, and nobody else's.

mod common;

use scorsese_core::{Fps, Project};
use scorsese_server::accounts::users;
use scorsese_server::db::{self, UserId};
use scorsese_server::projects::media::materialise;
use scorsese_server::projects::{self, ProjectError, ProjectFiles};
use sqlx::postgres::PgPool;

fn files(of: &[(&str, &str)]) -> ProjectFiles {
    let mut files = ProjectFiles::default();
    for (path, text) in of {
        files.insert(path, (*text).to_owned()).expect("a kept path");
    }
    files
}

async fn account(pool: &PgPool, email: &str) -> UserId {
    users::create(pool, email, "password one")
        .await
        .expect("the account is created")
}

#[sqlx::test]
async fn the_files_follow_every_save_under_the_documents_revision(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = db::member_pool(&pool).await.unwrap();
    let project = Project::new("p", Fps::default());
    let id = projects::create(&members, ana, &project).await.unwrap().id;
    let (_, none) = projects::open_with_files(&members, ana, id).await.unwrap();
    assert!(none.is_empty());

    let first = files(&[("recipes/theme.json", "{}"), ("script.md", "# Teaser")]);
    let at = projects::save_with_files(&members, ana, id, 1, &project, &first)
        .await
        .unwrap();
    assert_eq!(at, 2, "a change to the files alone is a revision");
    let (_, read) = projects::open_with_files(&members, ana, id).await.unwrap();
    assert_eq!(read, first);

    // One gone, one changed, one new: exactly the second set is kept.
    let second = files(&[
        ("recipes/theme.json", "{\"a\":1}"),
        ("recipes/kit.json", "{}"),
    ]);
    let stale = projects::save_with_files(&members, ana, id, 1, &project, &second).await;
    assert!(
        matches!(stale, Err(ProjectError::Conflict { current: 2 })),
        "{stale:?}"
    );
    projects::save_with_files(&members, ana, id, 2, &project, &second)
        .await
        .unwrap();
    let (_, read) = projects::open_with_files(&members, ana, id).await.unwrap();
    assert_eq!(read, second);

    // A save that names no files leaves them alone.
    projects::save(&members, ana, id, 3, &project)
        .await
        .unwrap();
    let (_, read) = projects::open_with_files(&members, ana, id).await.unwrap();
    assert_eq!(read, second);

    assert!(projects::delete(&members, ana, id).await.unwrap());
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM project_files")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0, "deleting the project deletes its files");
}

#[sqlx::test]
async fn another_users_files_are_not_there(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let bob = account(&pool, "bob@example.com").await;
    let members = db::member_pool(&pool).await.unwrap();
    let project = Project::new("p", Fps::default());
    let hers = projects::create(&members, ana, &project).await.unwrap().id;
    let kept = files(&[("script.md", "hers")]);
    projects::save_with_files(&members, ana, hers, 1, &project, &kept)
        .await
        .unwrap();

    let opened = projects::open_with_files(&members, bob, hers).await;
    assert!(matches!(opened, Err(ProjectError::NotFound)), "{opened:?}");
    let written = projects::save_with_files(&members, bob, hers, 2, &project, &files(&[])).await;
    assert!(
        matches!(written, Err(ProjectError::NotFound)),
        "{written:?}"
    );
    let (_, still) = projects::open_with_files(&members, ana, hers)
        .await
        .unwrap();
    assert_eq!(still, kept);
}

#[test]
fn a_laid_out_folder_holds_the_files_where_the_document_looks() {
    let mut project = Project::new("p", Fps::default());
    project.script = Some(scorsese_core::ProjectPath::new("notes/brief.md"));
    let kept = files(&[
        ("recipes/kit/snare.json", "{}"),
        ("notes/brief.md", "brief"),
    ]);
    let at = common::scratch("laid-files").join("p.scor");
    let folder = materialise(&project, &kept, &at, &|_: &str| None).unwrap();
    let read = |path: &str| std::fs::read_to_string(folder.root().join(path)).unwrap();
    assert_eq!(read("recipes/kit/snare.json"), "{}");
    assert_eq!(read("notes/brief.md"), "brief");
    assert!(!folder.root().join("notes/brief.md").is_symlink());
}
