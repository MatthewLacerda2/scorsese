//! Writing an edit onto an asset, as the caller sees it: what the reply says.

use crate::asset::AssetId;
use crate::authoring::fixture::project;
use crate::authoring::{Edit, set_asset};
use crate::color::Rgba;
use crate::shape::{Endpoint, Geometry, Point, Shape};

#[test]
fn a_colour_card_takes_its_one_field() {
    let mut project = project();
    let said = set_asset(
        &mut project,
        &AssetId::new("card"),
        &Edit {
            color: Some(Rgba::opaque(0xff, 0xcc, 0).into()),
            ..Edit::default()
        },
    )
    .expect("a repainted card");
    assert_eq!(said, vec!["color: #101820 → #ffcc00"]);
}

#[test]
fn a_box_takes_a_border_and_a_new_width_in_one_call() {
    let mut project = project();
    let said = set_asset(
        &mut project,
        &AssetId::new("box"),
        &Edit {
            stroke: Some(Rgba::BLACK),
            width: Some(0.5),
            ..Edit::default()
        },
    )
    .expect("a bordered box");
    assert_eq!(said, vec!["stroke: none → #000000", "width: 0.4 → 0.5"]);
}

/// An arrow is two endpoints and has no size at all, so a width on one is
/// refused by name rather than written where nothing reads it back.
#[test]
fn an_arrow_has_no_width() {
    let mut project = project();
    let arrow = Shape::outlined(
        Geometry::Arrow {
            from: Endpoint::from(Point::new(0.1, 0.1)),
            to: Endpoint::from(Point::new(0.9, 0.9)),
            curve: crate::shape::Curve::Straight,
            heads: crate::shape::Heads::End,
        },
        Rgba::WHITE,
    );
    crate::authoring::add_asset(
        &mut project,
        Some("pointer"),
        crate::authoring::Inline::Shape(arrow),
    )
    .expect("an arrow is a valid asset");
    let refused = set_asset(
        &mut project,
        &AssetId::new("pointer"),
        &Edit {
            width: Some(0.3),
            ..Edit::default()
        },
    );
    let problem = refused.expect_err("an arrow has no width");
    let said = problem.to_string();
    assert!(said.contains("`width` is not a field"), "{said}");
    assert!(said.contains("an arrow"), "{said}");
}

#[test]
fn an_icon_changes_symbol_and_colour_and_says_both() {
    let mut project = project();
    let said = set_asset(
        &mut project,
        &AssetId::new("mark"),
        &Edit {
            icon: Some("circle-play".to_owned()),
            color: Some(Rgba::BLACK.into()),
            ..Edit::default()
        },
    )
    .expect("another symbol");
    assert_eq!(
        said,
        vec![
            "icon: clapperboard → circle-play",
            "color: #ffffff → #000000"
        ]
    );
}
