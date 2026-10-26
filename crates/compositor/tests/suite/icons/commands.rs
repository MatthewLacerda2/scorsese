//! How many path commands an icon decodes to.
//!
//! `drawing.rs` asks every one of the seventeen hundred for a count above zero,
//! which is the check that finds a record the conversion dropped. It cannot tell
//! a count from a constant: an icon that answered *one command* for all of them
//! would pass it. So a handful of icons whose SVG is short enough to count by
//! hand are held to the exact number, which is what makes the count a decoding
//! rather than a claim.

use scorsese_compositor::icon;

fn commands(name: &str) -> Option<usize> {
    icon::find(name)
        .unwrap_or_else(|| panic!("'{name}' is a vendored icon"))
        .commands()
}

/// Three icons, three different numbers, each read off the vendored `d`
/// attribute: `minus` is `M5 12h14`, `chevron-right` is `m9 18 6-6-6-6`, and
/// `x` is two `M`-and-line paths.
#[test]
fn an_icons_count_is_the_commands_its_contours_hold() {
    assert_eq!(commands("minus"), Some(2), "a move and a line");
    assert_eq!(commands("chevron-right"), Some(3), "a move and two lines");
    assert_eq!(commands("x"), Some(4), "two moves and two lines");
}

/// Stroked and filled are added together, so an icon carrying a dot counts more
/// than the same drawing without one. `dice-5` is the rounded box plus five
/// pips; `square` is the box alone.
#[test]
fn the_filled_contours_are_counted_with_the_stroked_ones() {
    let box_alone = commands("square").expect("contours that decode");
    let with_pips = commands("dice-5").expect("contours that decode");
    assert!(
        with_pips > box_alone,
        "five pips on the same box: {box_alone} against {with_pips}"
    );
}
