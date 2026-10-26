//! Issuing and redeeming, against the store a local project keeps.

use scorsese_providers::quote::{
    LIFETIME_SECONDS, ProjectQuotes, Quote, Refused, Spend, generation, issue, redeem,
};

use super::sketched;

/// An instant to issue at, so expiry is arithmetic rather than waiting.
const NOW: i64 = 1_800_000_000;

/// The sketched project's quote, and a store for it.
fn quoted(label: &str) -> (std::path::PathBuf, Quote, ProjectQuotes) {
    let (dir, project) = sketched(label);
    let quote = generation(&project, &dir).expect("quote");
    let store = ProjectQuotes::new(&dir);
    (dir, quote, store)
}

#[test]
fn a_token_spends_exactly_what_it_was_issued_for() {
    let (dir, quote, store) = quoted("token-good");
    let issued = issue(&store, &quote, NOW).expect("issue");

    let redeemed = redeem(&store, &issued.token, &quote, NOW + 60).expect("redeem");
    assert_eq!(redeemed.cents, quote.cents());
    std::fs::remove_dir_all(dir).ok();
}

/// Single use: a second attempt — a retry, a replay, a second client — is a
/// token nobody holds any more.
#[test]
fn a_token_is_good_once() {
    let (dir, quote, store) = quoted("token-once");
    let issued = issue(&store, &quote, NOW).expect("issue");
    redeem(&store, &issued.token, &quote, NOW).expect("the first use");

    let again = redeem(&store, &issued.token, &quote, NOW);
    assert!(matches!(again, Err(Refused::Unknown { .. })), "{again:?}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_token_expires() {
    let (dir, quote, store) = quoted("token-expired");
    let issued = issue(&store, &quote, NOW).expect("issue");

    let late = redeem(&store, &issued.token, &quote, NOW + LIFETIME_SECONDS + 1);
    assert!(matches!(late, Err(Refused::Expired { ago: 1 })), "{late:?}");
    std::fs::remove_dir_all(dir).ok();
}

/// The whole point of binding: a brief edited after the yes is not what the
/// yes was given to.
#[test]
fn a_changed_brief_is_refused_and_says_what_it_would_cost_now() {
    let (dir, quote, store) = quoted("token-changed");
    let issued = issue(&store, &quote, NOW).expect("issue");

    let mut dearer = quote.clone();
    if let Some(charge) = dearer.items[0].charge.as_mut() {
        charge.brief = String::from("a different brief");
        charge.cents = 400;
    }
    let refused = redeem(&store, &issued.token, &dearer, NOW).expect_err("refused");
    let said = refused.to_string();
    assert!(matches!(refused, Refused::Changed { .. }), "{said}");
    assert!(said.contains("$0.97") && said.contains("$4.01"), "{said}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_token_for_one_kind_of_spending_cannot_pay_for_another() {
    let (dir, quote, store) = quoted("token-other");
    let issued = issue(&store, &quote, NOW).expect("issue");

    let mut design = quote.clone();
    design.spend = Spend::VoiceDesign;
    let refused = redeem(&store, &issued.token, &design, NOW);
    assert!(
        matches!(refused, Err(Refused::OtherSpend { .. })),
        "{refused:?}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A token is used as a file name, so anything that is not one never reaches
/// the file system — and a made-up one is simply not there.
#[test]
fn a_token_nobody_issued_is_refused() {
    let (dir, quote, store) = quoted("token-forged");
    for forged in ["quote-0123456789abcdef01234567", "../../project.json", ""] {
        let refused = redeem(&store, forged, &quote, NOW);
        assert!(matches!(refused, Err(Refused::Unknown { .. })), "{forged}");
    }
    assert!(dir.join("project.json").exists(), "nothing was touched");
    std::fs::remove_dir_all(dir).ok();
}

/// Issuing sweeps what has expired, so the directory holds only live quotes.
#[test]
fn expired_records_are_swept_on_the_next_issue() {
    let (dir, quote, store) = quoted("token-swept");
    issue(&store, &quote, NOW).expect("issue");
    issue(&store, &quote, NOW + LIFETIME_SECONDS + 1).expect("issue later");

    let left = std::fs::read_dir(dir.join("cache/quotes"))
        .expect("the store exists")
        .count();
    assert_eq!(left, 1, "the expired record is gone");
    std::fs::remove_dir_all(dir).ok();
}
