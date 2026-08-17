//! What a refusal offers when the name is not one of ours.
//!
//! Seventeen hundred names cannot be listed the way the two shipped faces can,
//! so the suggestions *are* the refusal's answer — a wrong name with no
//! suggestion beside it leaves an author nowhere to go.

use scorsese_compositor::icon;

/// A real name suggests itself, which is what makes the case-and-space
/// forgiveness worth having rather than a nicety.
#[test]
fn a_name_that_only_needed_tidying_comes_back_exactly() {
    for written in ["Clapperboard", " clapperboard ", "CLAPPERBOARD"] {
        assert_eq!(
            icon::nearest(written).first().copied(),
            Some("clapperboard"),
            "for {written:?}"
        );
    }
}

/// The typo case: two edits or fewer, and the right answer leads.
///
/// `clapboard` is deliberately *not* the example. It is three edits from
/// `clapperboard` and one from `clipboard`, so the honest suggestion is the
/// clipboard — which is the rule working, not failing: past two edits a
/// suggestion is a guess, and a wrong one is the thing a reader acts on without
/// checking.
#[test]
fn a_mistyped_name_suggests_the_one_that_was_meant() {
    for written in ["clapperbord", "clapperboad", "clapprboard"] {
        assert_eq!(
            icon::nearest(written).first().copied(),
            Some("clapperboard"),
            "for {written:?}"
        );
    }
}

/// The half-remembered case, which an edit distance alone never finds:
/// `circle-play` is seven edits from `play`.
#[test]
fn a_word_of_a_compound_finds_the_compounds() {
    let found = icon::nearest("play");
    assert!(
        found.contains(&"circle-play"),
        "expected the compounds, got {found:?}"
    );
}

/// And the start of one, which is the other half-remembered shape: nobody
/// writes `clapperboard` and stops at `clapper` by accident.
#[test]
fn the_start_of_a_word_finds_the_whole_of_it() {
    assert!(
        icon::nearest("clapper").contains(&"clapperboard"),
        "got {:?}",
        icon::nearest("clapper")
    );
}

/// A whole word of a compound, not a run of letters in the middle of one —
/// `row` sits inside `arrow-*` and means nothing there.
#[test]
fn a_run_of_letters_inside_a_word_is_not_a_match() {
    let found = icon::nearest("row");
    assert!(
        !found.iter().any(|name| name.starts_with("arrow")),
        "a coincidence inside a word is not a suggestion: {found:?}"
    );
}

/// Empty is an honest answer and reads as one: a name with no near neighbours
/// is not a typo, it is a symbol this set does not have.
#[test]
fn a_name_that_is_nothing_like_one_of_ours_suggests_nothing() {
    assert!(icon::nearest("qzxwvjkhgfdsplm").is_empty());
    assert!(icon::nearest("").is_empty());
    assert!(icon::nearest("   ").is_empty());
}

/// One line of a message, so the list has to stop somewhere — and it has to
/// stop in the same place every time, or the same wrong name would produce a
/// different refusal between two runs.
#[test]
fn the_list_is_short_and_always_in_the_same_order() {
    let found = icon::nearest("circle");
    assert!(found.len() <= 6, "got {found:?}");
    assert_eq!(found, icon::nearest("circle"));
}
