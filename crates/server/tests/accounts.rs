//! Accounts in the database: creating, ending sessions, deleting. Signing in
//! with Google is `tests/google`'s.

#[path = "../../../tools/scratch.rs"]
mod scratch;

use scorsese_server::AccountError;
use scorsese_server::accounts::{sessions, tokens, users};
use scorsese_server::storage;
use sqlx::postgres::PgPool;

#[sqlx::test]
async fn an_email_is_one_account_whatever_its_case(pool: PgPool) {
    users::create(&pool, "  Ana@Example.com ").await.unwrap();
    let error = users::create(&pool, "ana@example.COM").await.unwrap_err();
    assert!(
        matches!(error, AccountError::Taken(ref email) if email == "ana@example.com"),
        "{error}"
    );
    let listed = users::list(&pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].account.email, "ana@example.com");
    assert_eq!(listed[0].google_sub, None);
}

#[sqlx::test]
async fn logging_an_account_out_ends_its_sessions_alone(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let bia = users::create(&pool, "bia@example.com").await.unwrap();
    let phone = sessions::open(&pool, ana).await.unwrap();
    let laptop = sessions::open(&pool, ana).await.unwrap();
    let theirs = sessions::open(&pool, bia).await.unwrap();
    let token = tokens::issue(&pool, ana, "script").await.unwrap();

    assert_eq!(sessions::close_all(&pool, ana).await.unwrap(), 2);
    assert_eq!(sessions::find(&pool, &phone).await.unwrap(), None);
    assert_eq!(sessions::find(&pool, &laptop).await.unwrap(), None);
    assert_eq!(sessions::find(&pool, &theirs).await.unwrap(), Some(bia));
    assert!(tokens::find(&pool, &token.token).await.unwrap().is_some());
}

#[sqlx::test]
async fn deleting_an_account_removes_its_rows_and_its_files(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let bia = users::create(&pool, "bia@example.com").await.unwrap();
    let cookie = sessions::open(&pool, ana).await.unwrap();
    let token = tokens::issue(&pool, ana, "laptop").await.unwrap();

    let root = scratch::fresh("scorsese-533-delete", "");
    let files = storage::Storage::new(root.join("kept"), root.join("cache"));
    for user in [ana, bia] {
        for file in [
            files.library_file(user, "a", "mp4"),
            files.thumbnail(user, "a", "jpg"),
        ] {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, b"not really").unwrap();
        }
    }

    assert_eq!(
        users::delete(&pool, &files, "ana@example.com")
            .await
            .unwrap(),
        ana
    );
    assert_eq!(sessions::find(&pool, &cookie).await.unwrap(), None);
    assert_eq!(tokens::find(&pool, &token.token).await.unwrap(), None);
    assert!(!files.library_file(ana, "a", "mp4").exists());
    assert!(!files.thumbnail(ana, "a", "jpg").exists());
    assert!(files.library_file(bia, "a", "mp4").exists());
    assert!(files.thumbnail(bia, "a", "jpg").exists());
    assert_eq!(users::list(&pool).await.unwrap().len(), 1);

    let again = users::delete(&pool, &files, "ana@example.com")
        .await
        .unwrap_err();
    assert!(matches!(again, AccountError::NoSuchAccount(_)), "{again}");
    std::fs::remove_dir_all(&root).unwrap();
}
