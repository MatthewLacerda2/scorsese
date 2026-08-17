//! The `d` attribute: the grammar, and what it resolves to.

use scorsese_icon_vendor::geometry::Command;
use scorsese_icon_vendor::path::parse;

use crate::{ends_at, near};

/// Relative commands are resolved against where the pen is, and only the
/// absolute result reaches the blob — the compositor never adds anything up.
#[test]
fn a_relative_command_is_resolved_where_it_is_written() {
    let commands = parse("M 2 3 l 4 5").expect("a legal path");
    near(ends_at(commands[1]), (6.0, 8.0), "the line's end");
}

/// A move followed by more coordinates draws lines, which is the grammar's own
/// rule and not a guess. Lucide leans on it constantly.
#[test]
fn coordinates_after_a_move_are_lines() {
    let commands = parse("M0 0 1 1 2 2").expect("a legal path");
    assert!(matches!(commands[0], Command::Move(_)));
    assert!(matches!(commands[1], Command::Line(_)));
    near(ends_at(commands[2]), (2.0, 2.0), "the second line");
}

/// The shorthands carry the coordinate they are not given over from the pen.
#[test]
fn horizontal_and_vertical_keep_the_other_axis() {
    let commands = parse("M3 4 H10 v6").expect("a legal path");
    near(ends_at(commands[1]), (10.0, 4.0), "H holds y");
    near(ends_at(commands[2]), (10.0, 10.0), "v holds x");
}

/// `S` reflects the previous cubic's second control point through the current
/// point. Getting the reflection wrong makes a kink no test of endpoints alone
/// would ever see.
#[test]
fn a_smooth_cubic_reflects_the_last_control_point() {
    let commands = parse("M0 0 C1 2 3 4 5 5 S7 8 9 9").expect("a legal path");
    let Command::Cubic(first, _, _) = commands[2] else {
        panic!("a cubic");
    };
    near(first, (7.0, 6.0), "the reflected control point");
}

/// The same, for a quadratic's single control point.
#[test]
fn a_smooth_quadratic_reflects_too() {
    let commands = parse("M0 0 Q2 3 4 4 T8 8").expect("a legal path");
    let Command::Quad(control, _) = commands[2] else {
        panic!("a quadratic");
    };
    near(control, (6.0, 5.0), "the reflected control point");
}

/// A close returns the pen to where the contour began, so what follows is
/// measured from there.
#[test]
fn a_close_returns_to_the_start_of_the_contour() {
    let commands = parse("M5 5 l2 0 l0 2 z l1 1").expect("a legal path");
    near(ends_at(commands[4]), (6.0, 6.0), "relative to the start");
}

/// Arc flags are single digits and may be run together with the coordinates
/// after them — `a1 1 0 010 5` is two flags, a zero and a five, not the number
/// ten. Reading them as numbers is the classic way to mis-parse an SVG path,
/// and Lucide's rounded corners are all arcs.
#[test]
fn arc_flags_are_digits_not_numbers() {
    let commands = parse("M0 0a1 1 0 010 5").expect("a legal path");
    let end = ends_at(*commands.last().expect("the arc"));
    near(end, (0.0, 5.0), "the arc's end");
}

/// An arc becomes cubics, because that is all the rasteriser has. A half turn
/// is two of them: a quarter each, which is where the approximation is good.
#[test]
fn an_arc_becomes_quarter_turn_cubics() {
    let commands = parse("M0 0 A5 5 0 0 1 0 10").expect("a legal path");
    assert_eq!(commands.len(), 3, "a move and two cubics");
    assert!(
        commands[1..]
            .iter()
            .all(|c| matches!(c, Command::Cubic(..)))
    );
    near(ends_at(commands[2]), (0.0, 10.0), "the arc's end");
    // Halfway round a circle of radius 5 to the right of the chord.
    let Command::Cubic(_, _, middle) = commands[1] else {
        panic!("a cubic");
    };
    near(middle, (5.0, 5.0), "the top of the half circle");
}

/// A degenerate arc is a straight line — the spec's answer, and the one that
/// keeps a contour connected.
#[test]
fn an_arc_with_no_radius_is_a_line() {
    let commands = parse("M0 0 A0 0 0 0 1 4 4").expect("a legal path");
    assert!(matches!(commands[1], Command::Line(_)));
}

/// An unreadable path stops the vendoring run. Skipping it would ship an icon
/// missing a stroke, which looks like a drawing somebody meant.
#[test]
fn an_unknown_command_is_refused() {
    assert!(parse("M0 0 G4 4").is_err(), "'G' is not a command");
    assert!(parse("4 4").is_err(), "a path has to start with a command");
    assert!(parse("M0 0 A1 1 0 5 1 4 4").is_err(), "5 is not a flag");
}
