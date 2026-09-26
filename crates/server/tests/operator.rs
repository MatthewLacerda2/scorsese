//! The operator's commands read as the operator would read them.

use scorsese_server::accounts::{password, tokens, users};
use scorsese_server::operator::{self, TokenCommand, UserCommand};
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
async fn a_created_account_logs_in_with_the_printed_password(pool: PgPool) {
    let root = files();
    let email = "ana@example.com".to_owned();
    let output = operator::user(
        &pool,
        &root,
        UserCommand::Create {
            email: email.clone(),
        },
    )
    .await
    .unwrap();
    let printed = after(&output, "password: ");
    assert!(
        users::authenticate(&pool, &email, printed)
            .await
            .unwrap()
            .is_some()
    );

    let reset = operator::user(
        &pool,
        &root,
        UserCommand::ResetPassword {
            email: email.clone(),
        },
    )
    .await
    .unwrap();
    let new = after(&reset, &format!("{email}: "));
    assert_ne!(new, printed);
    assert!(
        users::authenticate(&pool, &email, new)
            .await
            .unwrap()
            .is_some()
    );

    let listed = operator::user(&pool, &root, UserCommand::List)
        .await
        .unwrap();
    assert!(listed.ends_with("\tana@example.com"), "{listed}");
}

#[sqlx::test]
async fn deleting_needs_yes(pool: PgPool) {
    let root = files();
    users::create(&pool, "ana@example.com", "password one")
        .await
        .unwrap();
    let delete = |yes| UserCommand::Delete {
        email: "ana@example.com".to_owned(),
        yes,
    };
    let refused = operator::user(&pool, &root, delete(false))
        .await
        .unwrap_err();
    assert!(matches!(refused, ServerError::Unconfirmed(_)), "{refused}");
    assert!(refused.to_string().contains("--yes"), "{refused}");
    assert_eq!(users::list(&pool).await.unwrap().len(), 1);

    operator::user(&pool, &root, delete(true)).await.unwrap();
    assert!(users::list(&pool).await.unwrap().is_empty());
}

#[sqlx::test]
async fn a_token_issued_by_the_operator_is_the_users(pool: PgPool) {
    let ana = users::create(&pool, "ana@example.com", "password one")
        .await
        .unwrap();
    let create = |email: &str| TokenCommand::Create {
        email: email.to_owned(),
        name: "script".to_owned(),
    };
    let output = operator::token(&pool, create("ana@example.com"))
        .await
        .unwrap();
    let token = output.lines().last().unwrap();
    assert_eq!(tokens::find(&pool, token).await.unwrap(), Some(ana));

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
fn a_generated_password_is_long_and_unambiguous() {
    let first = password::generate().unwrap();
    assert_eq!(first.len(), 20);
    assert!(!first.contains(['0', 'o', '1', 'l', 'i']), "{first}");
    assert_ne!(first, password::generate().unwrap());
}
