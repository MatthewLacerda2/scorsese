//! A `.scor` received from someone else may carry a link in its `cache/`
//! (#1715, #1718): no token is issued, read or spent through one.

#![cfg(unix)]

use scorsese_providers::quote::{Refused, issue, redeem};

use super::tokens::{NOW, quoted};

/// A `.scor` received from someone else may carry `cache/quotes` as a link
/// (#1715): no token is issued through it, and where it leads is untouched.
#[test]
fn a_quotes_folder_that_is_a_link_is_never_written_through() {
    let (dir, quote, store) = quoted("token-linked");
    let (outside, _) = crate::common::project("token-linked-outside");
    std::fs::create_dir_all(dir.join("cache")).expect("the cache folder");
    std::os::unix::fs::symlink(&outside, dir.join("cache/quotes")).expect("a link");
    let before = std::fs::read_dir(&outside).expect("outside").count();

    assert!(issue(&store, &quote, NOW).is_err(), "issued through a link");
    assert_eq!(
        std::fs::read_dir(&outside).expect("outside").count(),
        before
    );
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}

/// A token kept before `cache/quotes` became a link is not read or spent
/// through it (#1715): redeeming it is refused as unknown, and the record
/// where the link leads is left as it was.
#[test]
fn a_token_behind_a_linked_quotes_folder_is_never_spent_through_it() {
    let (dir, quote, store) = quoted("token-relinked");
    let issued = issue(&store, &quote, NOW).expect("issue");
    let (outside, _) = crate::common::project("token-relinked-outside");
    let moved = outside.join("quotes");
    std::fs::rename(dir.join("cache/quotes"), &moved).expect("moved out");
    std::os::unix::fs::symlink(&moved, dir.join("cache/quotes")).expect("a link");

    let again = redeem(&store, &issued.token, &quote, NOW);

    assert!(matches!(again, Err(Refused::Unknown { .. })), "{again:?}");
    assert_eq!(std::fs::read_dir(&moved).expect("outside").count(), 1);
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}

/// A `.scor` received from someone else may carry `cache/` itself as a link
/// (#1718): no token is issued through it, and where it leads is untouched.
#[test]
fn a_linked_cache_folder_is_never_written_through() {
    let (dir, quote, store) = quoted("token-linked-cache");
    let (outside, _) = crate::common::project("token-linked-cache-outside");
    let _ = std::fs::remove_dir_all(dir.join("cache"));
    std::os::unix::fs::symlink(&outside, dir.join("cache")).expect("a link");
    let before = std::fs::read_dir(&outside).expect("outside").count();

    assert!(issue(&store, &quote, NOW).is_err(), "issued through a link");
    assert_eq!(
        std::fs::read_dir(&outside).expect("outside").count(),
        before
    );
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}

/// A token kept before `cache/` became a link is not read or spent through it
/// (#1718): redeeming it is refused as unknown, and the record where the link
/// leads is left as it was.
#[test]
fn a_token_behind_a_linked_cache_folder_is_never_spent_through_it() {
    let (dir, quote, store) = quoted("token-relinked-cache");
    let issued = issue(&store, &quote, NOW).expect("issue");
    let (outside, _) = crate::common::project("token-relinked-cache-outside");
    let moved = outside.join("moved");
    std::fs::rename(dir.join("cache"), &moved).expect("moved out");
    std::os::unix::fs::symlink(&moved, dir.join("cache")).expect("a link");

    let again = redeem(&store, &issued.token, &quote, NOW);

    assert!(matches!(again, Err(Refused::Unknown { .. })), "{again:?}");
    assert_eq!(
        std::fs::read_dir(moved.join("quotes"))
            .expect("outside")
            .count(),
        1
    );
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(outside).ok();
}
