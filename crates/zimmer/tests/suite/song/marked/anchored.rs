//! Anchors by index and by name, around markers.

use scorsese_zimmer::song::Anchor;

use super::{marked, render, unmarked};

/// An anchor by index counts entries that play, so marking up a song keeps
/// every anchor on the downbeat it pinned.
#[test]
fn an_anchor_by_index_lands_where_it_did_before_markers() {
    let mut before = unmarked();
    before.anchors = vec![Anchor::at(3, 3.3)];
    let mut after = marked(&[(0, "intro"), (2, "verse"), (4, "chorus")]);
    after.anchors = before.anchors.clone();
    after.validate().expect("valid");
    assert_eq!(render(&after), render(&before));
}

/// An anchor by name pins that section's first downbeat: `verse` starts at
/// entry 2 of the entries that play.
#[test]
fn an_anchor_by_name_lands_on_its_sections_downbeat() {
    let mut by_index = unmarked();
    by_index.anchors = vec![Anchor::at(2, 2.4)];
    let mut by_name = marked(&[(0, "intro"), (2, "verse")]);
    by_name.anchors = vec![Anchor::named("verse", 2.4)];
    by_name.validate().expect("valid");
    assert_eq!(render(&by_name), render(&by_index));
    let anchor: Anchor =
        serde_json::from_str(r#"{ "section": "verse", "seconds": 2.4 }"#).expect("parses");
    assert_eq!(anchor, by_name.anchors[0]);
}

/// A name no marker carries is refused, saying so.
#[test]
fn an_anchor_naming_no_section_is_refused() {
    let mut song = marked(&[(0, "intro")]);
    song.anchors = vec![Anchor::named("bridge", 2.0)];
    let said = song.validate().expect_err("refused").to_string();
    assert!(said.contains("section bridge names no section"), "{said}");
}

/// A `section` that is neither a number nor a name says what it takes.
#[test]
fn an_anchor_section_of_the_wrong_type_says_what_it_takes() {
    let error = serde_json::from_str::<Anchor>(r#"{ "section": true, "seconds": 1 }"#)
        .expect_err("refused")
        .to_string();
    assert!(error.contains("or a section's name"), "{error}");
}
