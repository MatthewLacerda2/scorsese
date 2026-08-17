//! What an `icon` asset must and must not carry.
//!
//! Deliberately nothing here about whether the *name* is a symbol: the
//! catalogue is a set of property values and lives in the compositor, so
//! `scorsese check` answers that and this cannot. What is checkable from the
//! document is the block's presence and its two numbers.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    Asset, AssetField as F, AssetKind, AssetProblem as E, DEFAULT_ICON_STROKE_WIDTH, Icon,
    IconProblem as I, Rgba, ValidationError as V,
};

/// An icon asset in an otherwise valid project.
fn with_an_icon(icon: Option<Icon>) -> scorsese_core::Project {
    let mut p = project();
    p.assets.push(Asset {
        icon,
        ..Asset::icon(
            asset_id("badge"),
            Icon::new("clapperboard", 0.12, Rgba::WHITE),
        )
    });
    p
}

fn drawn(icon: Icon) -> scorsese_core::Project {
    with_an_icon(Some(icon))
}

#[test]
fn an_icon_asset_is_valid_on_its_own() {
    assert_eq!(
        drawn(Icon::new("clapperboard", 0.12, Rgba::WHITE)).validate(),
        Ok(())
    );
}

/// A name nothing ships is not this crate's to refuse — see the module doc.
#[test]
fn a_name_no_catalogue_has_is_not_a_document_problem() {
    assert_eq!(
        drawn(Icon::new("no-such-symbol", 0.1, Rgba::WHITE)).validate(),
        Ok(())
    );
}

#[test]
fn an_icon_asset_needs_an_icon() {
    assert_only_problem(
        &with_an_icon(None),
        E::MissingField {
            asset: asset_id("badge"),
            field: F::Icon,
            kind: AssetKind::Icon,
        },
    );
}

#[test]
fn nothing_but_an_icon_asset_may_carry_one() {
    let mut p = project();
    asset_mut(&mut p, "logo").icon = Some(Icon::new("clapperboard", 0.1, Rgba::WHITE));
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("logo"),
            field: F::Icon,
            kind: AssetKind::Image,
        },
    );
}

#[test]
fn an_icon_needs_a_size() {
    for size in [0.0, -0.2, f64::INFINITY] {
        assert_only_problem(
            &drawn(Icon::new("clapperboard", size, Rgba::WHITE)),
            E::Icon(I::NotSized {
                asset: asset_id("badge"),
                size,
            }),
        );
    }
}

/// An icon is only its line, so a thickness of nothing is an empty layer —
/// which looks exactly like a layer that failed to render.
#[test]
fn an_icon_needs_a_thickness() {
    for width in [0.0, -0.5, f64::INFINITY] {
        assert_only_problem(
            &drawn(Icon::new("clapperboard", 0.12, Rgba::WHITE).weighing(width)),
            E::Icon(I::NoThickness {
                asset: asset_id("badge"),
                width,
            }),
        );
    }
}

/// Named on its own because the two ends of the float range are what an
/// arithmetic mistake arrives at, and because `NaN` cannot be compared for
/// equality — so the assertion is on the variant rather than on the value.
#[test]
fn a_measurement_that_is_not_a_number_is_refused_too() {
    let sized = problems(&drawn(Icon::new("clapperboard", f64::NAN, Rgba::WHITE)));
    assert!(
        matches!(sized.as_slice(), [V::Asset(E::Icon(I::NotSized { .. }))]),
        "got {sized:?}"
    );
    let thick = problems(&drawn(
        Icon::new("clapperboard", 0.12, Rgba::WHITE).weighing(f64::NAN),
    ));
    assert!(
        matches!(thick.as_slice(), [V::Asset(E::Icon(I::NoThickness { .. }))]),
        "got {thick:?}"
    );
}

/// The thickness a document gets by leaving `stroke_width` out is the one every
/// symbol was drawn at, and it is a fraction of the icon rather than the frame.
#[test]
fn an_omitted_thickness_is_lucides_own() {
    let json = r##"{ "id": "badge", "kind": "icon",
                     "icon": { "name": "clapperboard", "size": 0.12, "color": "#ffffffff" } }"##;
    let asset: Asset = serde_json::from_str(json).expect("an icon asset parses");
    let icon = asset.icon.expect("the block is there");
    assert_eq!(icon.stroke_width, DEFAULT_ICON_STROKE_WIDTH);
    assert!((icon.stroke_width - 2.0 / 24.0).abs() < f64::EPSILON);
}
