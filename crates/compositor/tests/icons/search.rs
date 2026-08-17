//! Finding an icon by a word, which is the only way anyone reaches this set.
//!
//! Every case here is one an assistant hits on its first try: a word that is
//! not in the name it wants, a name it already knows, a word so broad the answer
//! has to be capped, and a word that finds nothing at all.

use scorsese_compositor::icon;

/// The whole point of searching tags: not one of these words is in the name.
#[test]
fn a_word_the_name_does_not_contain_still_finds_the_icon() {
    for word in ["film", "cinema", "movie", "entertainment"] {
        assert!(
            icon::search(word).names.contains(&"clapperboard"),
            "`{word}` did not find the clapperboard"
        );
    }
}

/// A fragment works, because the caller is guessing and a whole word is a
/// guess that has to be right.
#[test]
fn a_fragment_of_a_name_matches_it() {
    assert!(icon::search("clap").names.contains(&"clapperboard"));
    assert!(icon::search("perboa").names.contains(&"clapperboard"));
}

/// Case and stray spaces are the caller's, not the catalogue's.
#[test]
fn the_query_is_tidied_and_case_does_not_matter() {
    let found = icon::search("  CAMERA ");
    assert_eq!(found.query, "camera");
    assert_eq!(found.names, icon::search("camera").names);
}

/// A caller confirming a name it already has should read one word, not ten.
#[test]
fn an_exact_name_comes_first() {
    assert_eq!(icon::search("film").names.first().copied(), Some("film"));
    assert_eq!(icon::search("video").names.first().copied(), Some("video"));
}

/// Then names, then tags — the order of the evidence, so the icons actually
/// called something like the word are read before the ones merely filed under
/// it.
#[test]
fn names_come_before_the_icons_a_tag_matched() {
    let found = icon::search("camera");
    let named = |name: &str| {
        found
            .names
            .iter()
            .position(|found| *found == name)
            .unwrap_or_else(|| panic!("`{name}` is in the answer"))
    };
    assert!(named("switch-camera") < named("clapperboard"));
}

/// A broad word matches most of the set, and the answer says how many there
/// were as well as how many it is showing. A truncation that printed like a
/// complete answer would be wrong rather than short.
#[test]
fn a_broad_word_is_capped_and_says_so() {
    let found = icon::search("arrow");
    assert!(found.is_capped(), "`arrow` matches more than it shows");
    assert!(found.total > found.names.len());
    let said = found.to_string();
    assert!(said.contains(&found.total.to_string()), "{said}");
    assert!(said.contains(&found.names.len().to_string()), "{said}");
}

/// And a word that matches a handful is not capped, and says the one number.
#[test]
fn a_narrow_word_shows_everything_it_found() {
    let found = icon::search("cinema");
    assert!(!found.is_capped(), "{found}");
    assert_eq!(found.total, found.names.len());
}

/// Nothing found is an answer, and it reads as one.
#[test]
fn a_word_that_matches_nothing_says_what_was_searched() {
    let said = icon::search("qzxwvjkhgfdsplm").to_string();
    assert!(said.contains("No icon matches"), "{said}");
    assert!(said.contains("tags"), "{said}");
}

/// Everything is not an answer to *which icon is this*, so an empty query finds
/// nothing rather than the whole catalogue.
#[test]
fn an_empty_query_finds_nothing() {
    for query in ["", "   "] {
        assert_eq!(icon::search(query).total, 0, "for {query:?}");
    }
}

/// Every name that comes back is one the catalogue answers to — which is what
/// makes it safe to write straight into an `icon` asset.
#[test]
fn every_name_answered_is_a_name_that_draws() {
    for name in icon::search("music").names {
        assert!(icon::find(name).is_some(), "`{name}` is not findable");
    }
}

/// The same word always answers the same way, in the same order: a listing that
/// reordered itself between runs would be a diff nobody wrote.
#[test]
fn the_answer_is_the_same_every_time() {
    assert_eq!(icon::search("play").names, icon::search("play").names);
}
