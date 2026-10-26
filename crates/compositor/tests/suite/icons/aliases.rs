//! The name an icon used to have, which is the likeliest wrong guess there is.
//!
//! Upstream renames things, so the word the rest of the world uses is often a
//! name Lucide has retired. Each case here is the same shape: the retired name
//! **finds** the icon, says that it is retired, and is still not a name anything
//! will draw.

use scorsese_compositor::icon;

/// The three that are worth the format change: a word nothing else in the record
/// carries, and the one everybody types.
#[test]
fn a_retired_name_finds_the_icon_that_replaced_it() {
    for (typed, wanted) in [
        ("unlock", "lock-open"),
        ("file-json", "file-braces"),
        ("text-select", "square-dashed-text"),
    ] {
        let found = icon::search(typed);
        assert!(
            found.names().contains(&wanted),
            "`{typed}` did not find `{wanted}`: {found}"
        );
    }
}

/// Marked, because an unexplained hit on a word that is nowhere in the icon
/// reads exactly like the fuzziness this search refuses.
#[test]
fn a_hit_on_a_retired_name_says_so() {
    let found = icon::search("unlock");
    let hit = found
        .hits
        .iter()
        .find(|hit| hit.name == "lock-open")
        .expect("`unlock` finds the lock-open");
    assert_eq!(hit.formerly, Some("unlock"));
    let said = found.to_string();
    assert!(said.contains("lock-open (formerly unlock)"), "{said}");
    assert!(said.contains("write the name before it"), "{said}");
}

/// And nothing else is: a hit on a name, a tag or a category has no former name
/// beside it, so the explanation appears exactly as often as it is true.
#[test]
fn a_hit_on_something_current_is_not_marked() {
    let found = icon::search("cinema");
    assert!(
        found.hits.iter().all(|hit| hit.formerly.is_none()),
        "{found}"
    );
    assert!(!found.to_string().contains("formerly"), "{found}");
}

/// A retired name is the weakest evidence in the record, so it sorts after every
/// current one: somebody who typed a name upstream still uses must never be
/// pushed down the list by one it dropped.
#[test]
fn retired_names_sort_after_every_current_match() {
    // `unlock` is a tag on five keys and locks that are still called that, and
    // the retired name of two more.
    let found = icon::search("unlock");
    let last_current = found
        .hits
        .iter()
        .rposition(|hit| hit.formerly.is_none())
        .expect("current matches for `unlock`");
    let first_retired = found
        .hits
        .iter()
        .position(|hit| hit.formerly.is_some())
        .expect("retired matches for `unlock`");
    assert!(last_current < first_retired, "{found}");
}

/// Findable, never writable. The catalogue has one name per icon, and this is
/// the line the whole feature sits behind.
#[test]
fn a_retired_name_is_not_a_name_anything_draws() {
    assert!(icon::find("unlock").is_none(), "`unlock` is not a name");
    let replacement = icon::find("lock-open").expect("the one that replaced it");
    // The blob carries the retired name, and the reader reads it back whole:
    // the record grew a field, and the fields after it still line up.
    assert_eq!(replacement.aliases(), ["unlock"]);
    // Every answer stays writable, retired hits included — the name that comes
    // back is the catalogue's, and the former one is only the explanation.
    for name in icon::search("unlock").names() {
        assert!(icon::find(name).is_some(), "`{name}` is not findable");
    }
}

/// The whole reason 247 aliases were vendored rather than one: the tree carries
/// them and the blob has to still be carrying them.
#[test]
fn the_set_keeps_the_names_it_was_vendored_with() {
    let retired = icon::all()
        .iter()
        .filter(|icon| !icon.aliases().is_empty())
        .count();
    assert_eq!(retired, 247, "Lucide 1.31.0 retired names for this many");
}
