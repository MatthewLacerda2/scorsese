//! How a `fill` reads and writes: the colour string it always was, and the
//! gradient object beside it.
//!
//! The string form is the one every existing project is written in, so the
//! first test here is the promise that it still parses to exactly what it did.

use scorsese_core::{Fill, Linear, Point, Radial, Rgba, Stop};
use serde_json::json;

fn read(value: serde_json::Value) -> Result<Fill, String> {
    serde_json::from_value(value).map_err(|problem| problem.to_string())
}

#[test]
fn a_colour_string_parses_as_it_always_did_and_writes_back_the_same() {
    let fill = read(json!("#0b1020")).expect("a colour");
    assert_eq!(fill, Fill::Solid(Rgba::opaque(0x0b, 0x10, 0x20)));
    assert_eq!(
        serde_json::to_value(&fill).expect("written"),
        json!("#0b1020")
    );
    assert!(
        read(json!("#0b10"))
            .expect_err("too short")
            .contains("0b10")
    );
}

#[test]
fn a_linear_gradient_reads_its_angle_and_stops_and_round_trips() {
    let written = json!({ "linear": { "angle": 90.0,
        "stops": [["#0b1020", 0.0], ["#1b2440", 1.0]] } });
    let fill = read(written.clone()).expect("a gradient");
    assert_eq!(
        fill,
        Fill::Linear(Linear {
            angle: 90.0,
            stops: vec![
                Stop::new(Rgba::opaque(0x0b, 0x10, 0x20), 0.0),
                Stop::new(Rgba::opaque(0x1b, 0x24, 0x40), 1.0),
            ],
        })
    );
    assert_eq!(serde_json::to_value(&fill).expect("written"), written);
}

/// Top to bottom, and the middle: what CSS and every gradient tool default to.
#[test]
fn the_angle_and_the_centre_have_defaults() {
    let stops = json!([["#000000", 0], ["#ffffff", 1]]);
    let Fill::Linear(linear) = read(json!({ "linear": { "stops": stops } })).expect("linear")
    else {
        panic!("not linear");
    };
    assert_eq!(linear.angle, 180.0);
    let Fill::Radial(radial) =
        read(json!({ "radial": { "radius": 0.8, "stops": stops } })).expect("radial")
    else {
        panic!("not radial");
    };
    assert_eq!(radial.center, Point::new(0.5, 0.5));
}

#[test]
fn a_radial_gradient_round_trips() {
    let fill = Fill::Radial(Radial {
        center: Point::new(0.5, 0.45),
        radius: 0.8,
        stops: vec![
            Stop::new(Rgba::opaque(0x1b, 0x24, 0x40), 0.0),
            Stop::new(Rgba::new(0, 0, 0, 0x80), 1.0),
        ],
    });
    let written = serde_json::to_value(&fill).expect("written");
    assert_eq!(written["radial"]["stops"][1], json!(["#00000080", 1.0]));
    assert_eq!(read(written).expect("read back"), fill);
}

/// A misspelling is named, never guessed at: a gradient kind this build does
/// not draw, and a field a gradient does not have.
#[test]
fn an_unknown_gradient_or_field_is_refused_by_name() {
    let conic = read(json!({ "conic": { "stops": [] } })).expect_err("no conic");
    assert!(conic.contains("conic"), "{conic}");
    let stray = read(json!({ "linear": { "angel": 90, "stops": [] } })).expect_err("typo");
    assert!(stray.contains("angel"), "{stray}");
    assert!(read(json!(12)).is_err(), "a number is neither form");
}

#[test]
fn a_gradient_describes_itself_by_kind_and_colours() {
    let fill = read(json!({ "linear": { "angle": 90,
        "stops": [["#000000", 0], ["#ffffff", 1]] } }))
    .expect("a gradient");
    assert_eq!(fill.to_string(), "linear 90° gradient #000000 → #ffffff");
    assert_eq!(fill.solid(), None);
    assert_eq!(Fill::from(Rgba::WHITE).to_string(), "#ffffff");
}
