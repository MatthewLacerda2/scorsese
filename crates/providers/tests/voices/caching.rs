//! The cache: what it saves, when it is skipped, and what it rescues.

use scorsese_providers::voices::{Freshness, More, builtin, cached, library};

use super::fake::Fake;
use super::{root, voice};

/// The property the cache exists for: browsing a list twice touches the network
/// once.
#[test]
fn a_second_listing_does_not_ask_again() {
    let dir = root("voices-cache");
    let fake = Fake::listing(vec![voice("v1", "Ana")]);

    let first = builtin(&dir, &fake, false).expect("the first listing");
    assert_eq!(first.freshness, Freshness::Fetched);

    let again = builtin(&dir, &fake, false).expect("the second listing");
    assert_eq!(fake.calls(), 1, "the cache did not hold");
    assert_eq!(again.voices, first.voices);
    assert!(matches!(
        again.freshness,
        Freshness::Cached { refusal: None, .. }
    ));
}

/// `refresh` is the plain way past a cache that is current, and it has to
/// actually reach the provider — a refresh that served the cache would be a
/// button that does nothing.
#[test]
fn refresh_asks_again_even_when_the_cache_is_current() {
    let dir = root("voices-refresh");
    let fake = Fake::listing(vec![voice("v1", "Ana")]);

    builtin(&dir, &fake, false).expect("the first listing");
    let refreshed = builtin(&dir, &fake, true).expect("the refreshed listing");

    assert_eq!(fake.calls(), 2);
    assert_eq!(refreshed.freshness, Freshness::Fetched);
}

/// Two searches are two lists. A shared cache file would have the second
/// overwriting the first, and the answer to *Portuguese voices* would depend on
/// what somebody searched for before it.
#[test]
fn two_searches_do_not_overwrite_each_other() {
    let dir = root("voices-filters");
    let portuguese = Fake::listing(vec![voice("pt-1", "Ana")]);
    let english = Fake::listing(vec![voice("en-1", "Grace")]);
    let filter = |language: &str| scorsese_providers::voices::Filters {
        language: Some(String::from(language)),
        ..scorsese_providers::voices::Filters::default()
    };

    library(&dir, &portuguese, &filter("pt"), false).expect("the pt search");
    library(&dir, &english, &filter("en"), false).expect("the en search");

    let again = library(&dir, &portuguese, &filter("pt"), false).expect("the pt search again");
    assert_eq!(again.voices, vec![voice("pt-1", "Ana")]);
    assert_eq!(portuguese.calls(), 1, "the pt list was fetched twice");
}

/// A search that was cut short has to stay cut short once it is saved. The
/// cache is served for a week, so a copy that lost the caveat would be the
/// silent cap with a longer life than the reply that produced it.
#[test]
fn a_cached_search_is_still_reported_as_truncated() {
    let dir = root("voices-truncated");
    let mut capped = Fake::listing(vec![voice("pt-1", "Ana")]);
    capped.more = Some(More { total: Some(4274) });
    let filter = scorsese_providers::voices::Filters {
        language: Some(String::from("pt")),
        ..scorsese_providers::voices::Filters::default()
    };

    let fetched = library(&dir, &capped, &filter, false).expect("the search");
    assert_eq!(fetched.more, Some(More { total: Some(4274) }));

    let again = library(&dir, &capped, &filter, false).expect("the search again");
    assert_eq!(capped.calls(), 1, "the cache did not hold");
    let said = again.summary("the Voice Library", "--refresh");
    assert!(said.contains("of 4,274 matching voices"), "{said}");
}

/// Offline, on a train, with a key that works: a saved list is a far better
/// answer than none, and the reply has to say which one it is.
#[test]
fn an_unreachable_provider_falls_back_to_the_cache_and_says_so() {
    let dir = root("voices-offline");
    builtin(&dir, &Fake::listing(vec![voice("v1", "Ana")]), false).expect("the first listing");

    let mut offline = Fake::listing(Vec::new());
    offline.unreachable = true;
    let answer = builtin(&dir, &offline, true).expect("the cached listing");

    assert_eq!(answer.voices, vec![voice("v1", "Ana")]);
    let Freshness::Cached { refusal, .. } = answer.freshness else {
        panic!("a fallback must not report itself as freshly read");
    };
    assert!(
        refusal.is_some_and(|why| why.contains("no route to host")),
        "the fallback did not say why the provider was not asked"
    );
}

/// With nothing cached there is nothing to fall back on, and the failure has to
/// arrive whole — the missing-permission advice is the entire value of it.
#[test]
fn with_no_cache_the_refusal_is_what_comes_back() {
    let dir = root("voices-unscoped");
    let mut unscoped = Fake::listing(Vec::new());
    unscoped.unscoped = true;

    let error = builtin(&dir, &unscoped, false).expect_err("an unscoped key cannot list");
    let said = error.to_string();
    assert!(said.contains("voices_read"), "{said}");
    assert!(
        said.contains("dashboard"),
        "the advice must point at the dashboard, not at .env: {said}"
    );
}

/// What a surface with no key can still offer. Both listings, as one list, out
/// of `cache/` and nothing else — and a voice that is in both is one voice.
#[test]
fn every_cached_listing_is_readable_without_a_key() {
    let dir = root("voices-cached");
    let filter = scorsese_providers::voices::Filters {
        language: Some(String::from("pt")),
        ..scorsese_providers::voices::Filters::default()
    };
    builtin(&dir, &Fake::listing(vec![voice("v1", "Ana")]), false).expect("the built-in listing");
    library(
        &dir,
        &Fake::listing(vec![voice("v1", "Ana"), voice("v2", "Bea")]),
        &filter,
        false,
    )
    .expect("the library search");

    let names: Vec<String> = cached(&dir).into_iter().map(|found| found.name).collect();
    assert_eq!(names, ["Ana", "Bea"], "deduplicated, and sorted by name");
}

/// A project nobody has listed voices in answers with an empty list rather than
/// a failure: *there are none cached* is an answer, and it is the one a window
/// has to be able to say plainly.
#[test]
fn a_project_with_nothing_cached_says_so_without_failing() {
    assert!(cached(&root("voices-none")).is_empty());
}
