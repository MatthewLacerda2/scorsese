//! The operator's commands read as the operator would read them, and every
//! one of them, and every flag, says what it is for.

#[path = "../../../cli/tests/suite/help/walk.rs"]
mod walk;

use clap::CommandFactory;
use scorsese_server::accounts::{sessions, tokens, users};
use scorsese_server::generations::Keys;
use scorsese_server::operator::{self, Cli, TokenCommand, UserCommand};
use scorsese_server::storage::Storage;
use scorsese_server::{AccountError, ServerError};
use sqlx::postgres::PgPool;

/// Where an account's files would be, if these tests gave it any.
fn files() -> Storage {
    let temp = std::env::temp_dir();
    Storage::new(
        temp.join("scorsese-operator-kept"),
        temp.join("scorsese-operator-cache"),
    )
}

/// The line after `label` in a command's output.
fn after<'a>(output: &'a str, label: &str) -> &'a str {
    output
        .split(label)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("no {label:?} in {output:?}"))
}

#[sqlx::test]
async fn an_account_is_created_linked_listed_and_given_a_session(pool: PgPool) {
    let root = files();
    let email = || "ana@example.com".to_owned();
    let run = |command| operator::user(&pool, &root, &Keys, command);
    let created = run(UserCommand::Create { email: email() }).await.unwrap();
    assert!(
        created.contains("linked when they sign in with Google"),
        "{created}"
    );
    let listed = run(UserCommand::List).await.unwrap();
    assert!(
        listed.ends_with("\tana@example.com\t-\tnot signed in yet"),
        "{listed}"
    );

    let link = |google_sub: &str| UserCommand::Link {
        email: email(),
        google_sub: google_sub.to_owned(),
    };
    run(link(" 1101 ")).await.unwrap();
    let listed = run(UserCommand::List).await.unwrap();
    assert!(listed.ends_with("\tana@example.com\t-\t1101"), "{listed}");

    let session = run(UserCommand::Session { email: email() }).await.unwrap();
    let cookie = after(&session, "scorsese_session=");
    let ana = users::find(&pool, "ana@example.com").await.unwrap();
    assert_eq!(sessions::find(&pool, cookie).await.unwrap(), Some(ana));
    let ended = run(UserCommand::Logout { email: email() }).await.unwrap();
    assert_eq!(ended, "ended 1 browser session(s) for ana@example.com");
    assert_eq!(sessions::find(&pool, cookie).await.unwrap(), None);
}

#[sqlx::test]
async fn deleting_needs_yes(pool: PgPool) {
    let root = files();
    users::create(&pool, "ana@example.com").await.unwrap();
    let delete = |yes| UserCommand::Delete {
        email: "ana@example.com".to_owned(),
        yes,
    };
    let refused = operator::user(&pool, &root, &Keys, delete(false))
        .await
        .unwrap_err();
    assert!(matches!(refused, ServerError::Unconfirmed(_)), "{refused}");
    assert!(refused.to_string().contains("--yes"), "{refused}");
    assert_eq!(users::list(&pool).await.unwrap().len(), 1);

    operator::user(&pool, &root, &Keys, delete(true))
        .await
        .unwrap();
    assert!(users::list(&pool).await.unwrap().is_empty());
}

#[sqlx::test]
async fn a_token_issued_by_the_operator_is_the_users(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com").await.unwrap();
    let create = |email: &str| TokenCommand::Create {
        email: email.to_owned(),
        name: "script".to_owned(),
    };
    let output = operator::token(&pool, create("ana@example.com"))
        .await
        .unwrap();
    let token = output.lines().last().unwrap();
    assert_eq!(
        tokens::find(&pool, token)
            .await
            .unwrap()
            .map(|(user, _)| user),
        Some(ana)
    );

    let nobody = operator::token(&pool, create("bia@example.com"))
        .await
        .unwrap_err();
    assert!(
        matches!(nobody, ServerError::Account(AccountError::NoSuchAccount(_))),
        "{nobody}"
    );
}

#[test]
fn an_email_is_normalised_and_loosely_checked() {
    use scorsese_server::accounts::normalize_email;
    assert_eq!(
        normalize_email(" Ana@Example.COM ").unwrap(),
        "ana@example.com"
    );
    for bad in [
        "",
        "ana",
        "@example.com",
        "ana@",
        "a@b@c",
        "ana maria@example.com",
    ] {
        assert!(normalize_email(bad).is_err(), "{bad:?} was accepted");
    }
}

#[test]
fn every_server_command_and_argument_has_help_text() {
    let found = walk::undocumented(Cli::command());
    assert!(
        found.is_empty(),
        "no help text on:\n  {}",
        found.join("\n  ")
    );
    let arguments = walk::arguments(Cli::command());
    assert!(
        arguments > 10,
        "walked only {arguments} arguments: the walk stopped reaching the tree"
    );
}
