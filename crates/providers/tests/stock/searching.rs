//! Searching: paging, filtering, and the 24-hour cache.

use scorsese_providers::stock::{
    Medium, Orientation, PER_REPLY, cache_dir, find, find_cached, search,
};

use crate::common::project;
use crate::fake::{Fake, query};

#[test]
fn a_search_is_asked_once_and_then_answered_from_the_cache() {
    let (root, _) = project("stock-cache");
    let cache = cache_dir(&root);
    let library = Fake::with(120);
    let first = search(&cache, &library, &query("sunrise"), 1).unwrap();
    assert_eq!(first.candidates.len(), PER_REPLY);
    assert!(first.fetched && first.more);
    assert!(first.summary().contains("from Fake"), "{}", first.summary());

    let again = search(&cache, &library, &query("Sunrise "), 2).unwrap();
    assert!(!again.fetched, "the same words are the same cached page");
    assert_eq!(again.candidates[0].id, 6);
    assert_eq!(library.pages.get(), 1);
}

#[test]
fn the_last_page_says_there_is_no_more() {
    let (root, _) = project("stock-last");
    let library = Fake::with(7);
    let found = search(&cache_dir(&root), &library, &query("cat"), 2).unwrap();
    assert_eq!(found.candidates.len(), 2);
    assert!(!found.more);
    assert_eq!(found.total, 7);
}

/// Pixabay cannot filter footage by orientation, so it is filtered here —
/// by measured size — reading further pages until the reply is full.
#[test]
fn vertical_footage_is_filtered_here_across_pages() {
    let (root, _) = project("stock-vertical");
    let mut library = Fake::with(200);
    library.vertical_every = 20;
    let asked = scorsese_providers::stock::Query {
        orientation: Some(Orientation::Vertical),
        ..query("city")
    };
    let found = search(&cache_dir(&root), &library, &asked, 1).unwrap();
    assert_eq!(found.candidates.len(), PER_REPLY);
    assert!(found.candidates.iter().all(|one| {
        let largest = one.largest().unwrap();
        largest.height > largest.width
    }));
    assert!(
        library.pages.get() > 1,
        "one page held too few vertical shots"
    );
}

#[test]
fn an_id_a_search_returned_is_found_without_asking_again() {
    let (root, _) = project("stock-find");
    let cache = cache_dir(&root);
    let library = Fake::with(20);
    search(&cache, &library, &query("cat"), 1).unwrap();
    assert!(find_cached(&cache, Medium::Video, 3).is_some());
    assert!(
        find_cached(&cache, Medium::Image, 3).is_none(),
        "ids are per medium"
    );
    find(&cache, &library, Medium::Video, 3).unwrap();
    assert_eq!(library.ones.get(), 0);

    find(&cache, &library, Medium::Video, 15).unwrap();
    find(&cache, &library, Medium::Video, 15).unwrap();
    assert_eq!(library.ones.get(), 0, "15 was on the cached page too");
    let missing = find(&cache, &library, Medium::Image, 99).unwrap_err();
    assert!(missing.to_string().contains("no image 99"), "{missing}");
}
