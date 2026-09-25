//! Accounts in the database: creating, logging in, resetting, deleting.

use scorsese_server::AccountError;
use scorsese_server::accounts::{sessions, tokens, users};
use scorsese_server::storage;
use sqlx::postgres::PgPool;

#[sqlx::test]
async fn an_email_is_one_account_whatever_its_case(pool: PgPool) {
    users::create(&pool, "  Ana@Example.com ", "password one")
        .await
        .unwrap();
    let error = users::create(&pool, "ana@example.COM", "other")
        .await
        .unwrap_err();
    assert!(
        matches!(error, AccountError::Taken(ref email) if email == "ana@example.com"),
        "{error}"
    );
    let listed = users::list(&pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].email, "ana@example.com");
}

#[sqlx::test]
async fn only_the_right_password_logs_in(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com", "correct horse")
        .await
        .unwrap();
    let login = |email: &'static str, password: &'static str| {
        let pool = pool.clone();
        async move { users::authenticate(&pool, email, password).await.unwrap() }
    };
    assert_eq!(login("ANA@example.com", "correct horse").await, Some(ana));
    assert_eq!(login("ana@example.com", "Correct horse").await, None);
    assert_eq!(login("nobody@example.com", "correct horse").await, None);
    assert_eq!(login("not an email", "correct horse").await, None);
}

#[sqlx::test]
async fn a_reset_password_works_and_logs_every_browser_out(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com", "old password")
        .await
        .unwrap();
    let cookie = sessions::open(&pool, ana).await.unwrap();
    assert_eq!(sessions::find(&pool, &cookie).await.unwrap(), Some(ana));

    users::set_password(&pool, "ana@example.com", "new password")
        .await
        .unwrap();
    assert_eq!(sessions::find(&pool, &cookie).await.unwrap(), None);
    let authenticate = |password| users::authenticate(&pool, "ana@example.com", password);
    assert_eq!(authenticate("old password").await.unwrap(), None);
    assert_eq!(authenticate("new password").await.unwrap(), Some(ana));

    let error = users::set_password(&pool, "bia@example.com", "x")
        .await
        .unwrap_err();
    assert!(matches!(error, AccountError::NoSuchAccount(_)), "{error}");
}

#[sqlx::test]
async fn changing_ones_own_password_needs_the_current_one(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com", "old password")
        .await
        .unwrap();
    let wrong = users::change_password(&pool, ana, "guess", "new password").await;
    assert!(
        matches!(wrong, Err(AccountError::WrongPassword)),
        "{wrong:?}"
    );
    let short = users::change_password(&pool, ana, "old password", "short").await;
    assert!(
        matches!(short, Err(AccountError::WeakPassword)),
        "{short:?}"
    );

    users::change_password(&pool, ana, "old password", "new password")
        .await
        .unwrap();
    let authenticated = users::authenticate(&pool, "ana@example.com", "new password").await;
    assert_eq!(authenticated.unwrap(), Some(ana));
}

#[sqlx::test]
async fn deleting_an_account_removes_its_rows_and_its_files(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com", "password one")
        .await
        .unwrap();
    let bia = users::create(&pool, "bia@example.com", "password two")
        .await
        .unwrap();
    let cookie = sessions::open(&pool, ana).await.unwrap();
    let token = tokens::issue(&pool, ana, "laptop").await.unwrap();

    let root = std::env::temp_dir().join(format!("scorsese-533-delete-{}", std::process::id()));
    for user in [ana, bia] {
        let directory = storage::user_directory(&root, user);
        std::fs::create_dir_all(directory.join("library")).unwrap();
        std::fs::write(directory.join("library/clip.mp4"), b"not really").unwrap();
    }

    assert_eq!(
        users::delete(&pool, &root, "ana@example.com")
            .await
            .unwrap(),
        ana
    );
    assert_eq!(sessions::find(&pool, &cookie).await.unwrap(), None);
    assert_eq!(tokens::find(&pool, &token.token).await.unwrap(), None);
    assert!(!storage::user_directory(&root, ana).exists());
    assert!(
        storage::user_directory(&root, bia)
            .join("library/clip.mp4")
            .exists()
    );
    assert_eq!(users::list(&pool).await.unwrap().len(), 1);

    let again = users::delete(&pool, &root, "ana@example.com")
        .await
        .unwrap_err();
    assert!(matches!(again, AccountError::NoSuchAccount(_)), "{again}");
    std::fs::remove_dir_all(&root).unwrap();
}
