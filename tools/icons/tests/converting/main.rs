//! The conversion, on inputs small enough to check by hand.
//!
//! The vendored set is checked end to end elsewhere — the compositor's suite
//! draws all 1767 of them out of the committed blob. What these ask is the
//! thing that suite cannot: whether a curve is in the *right place*, which on
//! seventeen hundred icons nobody can read off a raster.

mod elements;
mod paths;

use scorsese_icon_vendor::geometry::{Command, Point};

/// A whole Lucide icon, header and all, around whatever elements are given.
pub(crate) fn icon(elements: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"
           viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
           {elements}
         </svg>"#
    )
}

/// Asserts a point is where it should be, to a thousandth of a unit — finer
/// than the three decimals upstream writes its coordinates to.
#[track_caller]
pub(crate) fn near(found: Point, expected: Point, what: &str) {
    let close = (found.0 - expected.0).abs() < 0.001 && (found.1 - expected.1).abs() < 0.001;
    assert!(close, "{what}: expected {expected:?}, found {found:?}");
}

/// The end point of a command, which is the last one it names.
#[track_caller]
pub(crate) fn ends_at(command: Command) -> Point {
    match command {
        Command::Move(to) | Command::Line(to) => to,
        Command::Cubic(_, _, to) | Command::Quad(_, to) => to,
        Command::Close => panic!("a close names no point"),
    }
}
