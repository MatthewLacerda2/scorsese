//! Which account a Google identity opens (#952, *Decisions* 1 and 2).

use scorsese_providers::sign_in::Identity;
use scorsese_server::AccountError;
use scorsese_server::accounts::users::{self, SignedIn};
use sqlx::postgres::PgPool;

fn google(sub: &str, email: &str) -> Identity {
    Identity {
        sub: sub.to_owned(),
        email: email.to_owned(),
        name: None,
    }
}

fn named(sub: &str, email: &str, name: &str) -> Identity {
    Identity {
        name: Some(name.to_owned()),
        ..google(sub, email)
    }
}

#[sqlx::test]
async fn the_name_google_gives_is_kept_and_follows_every_sign_in(pool: PgPool) {
    let made = users::create(&pool, "bia@example.com").await.unwrap();
    let name = || async { users::get(&pool, made).await.unwrap().name };
    assert_eq!(name().await, None, "no name before the first sign-in");
    let bia = named("3303", "bia@example.com", "Bia");
    let linked = users::sign_in(&pool, &bia).await.unwrap();
    assert_eq!(linked, SignedIn::Linked(made));
    assert_eq!(name().await.as_deref(), Some("Bia"));
    let renamed = named("3303", "bia@example.com", "Beatriz Lima");
    users::sign_in(&pool, &renamed).await.unwrap();
    assert_eq!(name().await.as_deref(), Some("Beatriz Lima"));
    // A sign-in carrying no name keeps the one stored.
    users::sign_in(&pool, &google("3303", "bia@example.com"))
        .await
        .unwrap();
    assert_eq!(name().await.as_deref(), Some("Beatriz Lima"));

    let caio = named("4404", "caio@example.com", "Caio");
    let SignedIn::Created(caio) = users::sign_in(&pool, &caio).await.unwrap() else {
        panic!("a new account")
    };
    let account = users::get(&pool, caio).await.unwrap();
    assert_eq!(account.name.as_deref(), Some("Caio"));
    let listed = users::list(&pool).await.unwrap();
    assert_eq!(listed[1].account.name.as_deref(), Some("Caio"));
}

#[sqlx::test]
async fn a_new_google_account_gets_a_new_account_once(pool: PgPool) {
    let first = users::sign_in(&pool, &google("1101", "Ana@Example.com"))
        .await
        .unwrap();
    let SignedIn::Created(ana) = first else {
        panic!("{first:?}")
    };
    let again = users::sign_in(&pool, &google("1101", "ana@example.com"))
        .await
        .unwrap();
    assert_eq!(again, SignedIn::Returning(ana));
    let listed = users::list(&pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].account.email, "ana@example.com");
    assert_eq!(listed[0].google_sub.as_deref(), Some("1101"));
}

#[sqlx::test]
async fn an_existing_account_is_linked_by_its_email_then_known_by_sub(pool: PgPool) {
    // The maintainer's account 1, made before Google sign-in existed.
    let made = users::create(&pool, "maintainer@example.com")
        .await
        .unwrap();
    let first = users::sign_in(&pool, &google("2202", "Maintainer@example.com")).await;
    assert_eq!(first.unwrap(), SignedIn::Linked(made));
    // Their Google address changes: the sub still finds them.
    let later = users::sign_in(&pool, &google("2202", "new@example.com")).await;
    assert_eq!(later.unwrap(), SignedIn::Returning(made));
    assert_eq!(users::list(&pool).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn an_account_linked_to_one_google_account_is_not_taken_by_another(pool: PgPool) {
    users::sign_in(&pool, &google("1101", "ana@example.com"))
        .await
        .unwrap();
    let other = users::sign_in(&pool, &google("9909", "ana@example.com")).await;
    assert!(
        matches!(other, Err(AccountError::LinkedElsewhere(ref email)) if email == "ana@example.com"),
        "{other:?}"
    );
}

#[sqlx::test]
async fn the_operator_links_by_hand_once_the_sub_is_free(pool: PgPool) {
    let old = users::create(&pool, "ana@work.example").await.unwrap();
    // Their first sign-in, under another address, made an empty account.
    let SignedIn::Created(stray) = users::sign_in(&pool, &google("1101", "ana@home.example"))
        .await
        .unwrap()
    else {
        panic!("a new account")
    };
    let taken = users::link(&pool, "ana@work.example", "1101").await;
    assert!(
        matches!(taken, Err(AccountError::SubTaken(id)) if id == stray.get()),
        "{taken:?}"
    );
    assert!(taken.unwrap_err().to_string().contains("user delete"));

    users::link(&pool, "ana@home.example", "other")
        .await
        .unwrap();
    assert_eq!(
        users::link(&pool, "ana@work.example", "1101")
            .await
            .unwrap(),
        old
    );
    let back = users::sign_in(&pool, &google("1101", "ana@home.example")).await;
    assert_eq!(back.unwrap(), SignedIn::Returning(old));
    let nobody = users::link(&pool, "bia@example.com", "3303").await;
    assert!(
        matches!(nobody, Err(AccountError::NoSuchAccount(_))),
        "{nobody:?}"
    );
}
