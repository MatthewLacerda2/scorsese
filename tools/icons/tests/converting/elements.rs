//! The seven elements, the two buckets, and everything else refused.

use scorsese_icon_vendor::blob::{self, Entry};
use scorsese_icon_vendor::geometry::Command;
use scorsese_icon_vendor::svg::parse;

use crate::{ends_at, icon, near};

/// A stroked element is the ordinary case, and where an icon's ink comes from.
#[test]
fn a_line_becomes_a_move_and_a_line() {
    let drawing = parse(&icon(r#"<line x1="1" y1="2" x2="3" y2="4" />"#)).expect("an icon");
    assert!(drawing.filled.is_empty(), "nothing is filled");
    assert!(matches!(drawing.stroked[0], Command::Move(_)));
    near(ends_at(drawing.stroked[1]), (3.0, 4.0), "the far end");
}

/// A circle is four cubics that come back to where they started.
#[test]
fn a_circle_closes_on_itself() {
    let drawing = parse(&icon(r#"<circle cx="12" cy="12" r="6" />"#)).expect("an icon");
    assert_eq!(drawing.stroked.len(), 6, "a move, four cubics and a close");
    near(ends_at(drawing.stroked[0]), (18.0, 12.0), "the start");
    near(ends_at(drawing.stroked[4]), (18.0, 12.0), "back to it");
    near(ends_at(drawing.stroked[2]), (6.0, 12.0), "the far side");
}

/// The dots — a dice pip, the dot of an exclamation mark — are filled, and
/// stroking one would draw a ring where a full stop belongs.
#[test]
fn a_filled_circle_goes_in_the_other_bucket() {
    let source = icon(r#"<circle cx="9" cy="9" r=".5" fill="currentColor" />"#);
    let drawing = parse(&source).expect("an icon");
    assert!(drawing.stroked.is_empty(), "nothing is stroked");
    assert_eq!(drawing.filled.len(), 6, "the dot");
}

/// A rectangle given one radius is rounded equally on both axes — SVG's rule,
/// and 394 of Lucide's rectangles rely on it.
#[test]
fn one_corner_radius_rounds_both_axes() {
    let square = parse(&icon(r#"<rect x="0" y="0" width="10" height="10" />"#)).expect("an icon");
    assert_eq!(square.stroked.len(), 5, "four corners and a close");
    let rounded = parse(&icon(
        r#"<rect x="0" y="0" width="10" height="10" rx="2" />"#,
    ))
    .expect("an icon");
    assert_eq!(rounded.stroked.len(), 10, "four of them are curves now");
    near(
        ends_at(rounded.stroked[0]),
        (2.0, 0.0),
        "in from the corner",
    );
}

/// A polygon closes and a polyline does not, which is the whole difference
/// between them.
#[test]
fn a_polygon_closes_and_a_polyline_does_not() {
    let open = parse(&icon(r#"<polyline points="1 2 3 4 5 6" />"#)).expect("an icon");
    assert_eq!(open.stroked.len(), 3, "a move and two lines");
    let closed = parse(&icon(r#"<polygon points="1,2 3,4 5,6" />"#)).expect("an icon");
    assert!(matches!(closed.stroked[3], Command::Close));
}

/// Anything outside the subset stops the run rather than being skipped. A `g`,
/// a `transform` or a gradient arriving upstream has to be a decision somebody
/// makes, not an icon that quietly draws three quarters of itself.
#[test]
fn what_is_not_in_the_subset_is_refused() {
    assert!(
        parse(&icon("<g><path d=\"M0 0\" /></g>")).is_err(),
        "a group"
    );
    assert!(parse(&icon(r#"<text x="1" y="2" />"#)).is_err(), "text");
    let coloured = icon(r##"<circle cx="9" cy="9" r="1" fill="#f00" />"##);
    assert!(parse(&coloured).is_err(), "a second colour");
    assert!(parse(&icon("")).is_err(), "an icon that draws nothing");
}

/// The header is checked, not assumed: one stroke colour and one thickness is
/// what makes the whole set drawable the one way, and an icon that opted out
/// of the conventions would come out wrong rather than differently.
#[test]
fn an_icon_that_is_not_drawn_lucides_way_is_refused() {
    let odd = icon(r#"<line x1="1" y1="2" x2="3" y2="4" />"#).replace("0 0 24 24", "0 0 48 48");
    assert!(parse(&odd).is_err(), "a viewBox of another size");
    let filled = icon(r#"<line x1="1" y1="2" x2="3" y2="4" />"#).replace(r#"fill="none""#, "");
    assert!(parse(&filled).is_err(), "no fill stated at all");
}

/// The blob says what it is before it says anything else, so a reader can
/// refuse bytes that are not a catalogue instead of misreading them.
#[test]
fn the_blob_announces_itself() {
    let entry = Entry {
        name: "dot".to_string(),
        tags: vec!["point".to_string()],
        categories: Vec::new(),
        aliases: vec!["pip".to_string()],
        drawing: parse(&icon(r#"<circle cx="9" cy="9" r="1" />"#)).expect("an icon"),
    };
    let bytes = blob::encode("1.31.0", &[entry]).expect("a legal catalogue");
    assert!(bytes.starts_with(blob::MAGIC), "the magic comes first");
    assert_eq!(bytes[blob::MAGIC.len()], blob::FORMAT, "then the format");
}
