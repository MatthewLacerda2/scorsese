//! What a `color` asset must and must not carry.
//!
//! The kind has no file and no content — a colour is the whole of it — so the
//! only thing validation can say about one is whether that colour is there,
//! and whether it is somewhere it does not belong.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{Asset, AssetField as F, AssetKind, AssetProblem as E, Rgba};

/// A colour asset in an otherwise valid project, on the video track the
/// fixture already has, built by the constructor a caller actually reaches for.
///
/// **Nothing here writes over the colour the constructor attached**, and that is
/// the point of the shape of it: a helper that took an `Option` and assigned it
/// afterwards would leave `Asset::color` asserted by nothing at all, and it
/// could stop attaching a colour with every test in this file still green.
fn with_a_colour(color: Rgba) -> scorsese_core::Project {
    let mut p = project();
    p.assets.push(Asset::color(asset_id("bg"), color));
    p
}

/// The one case that has to undo what the constructor did, and the only one.
fn without_a_colour() -> scorsese_core::Project {
    let mut p = with_a_colour(Rgba::BLACK);
    asset_mut(&mut p, "bg").color = None;
    p
}

#[test]
fn a_colour_asset_is_valid_on_its_own() {
    assert_eq!(with_a_colour(Rgba::BLACK).validate(), Ok(()));
}

#[test]
fn a_colour_asset_needs_a_colour() {
    assert_only_problem(
        &without_a_colour(),
        E::MissingField {
            asset: asset_id("bg"),
            field: F::Color,
            kind: AssetKind::Color,
        },
    );
}

#[test]
fn nothing_but_a_colour_asset_may_carry_a_colour() {
    let mut p = project();
    asset_mut(&mut p, "logo").color = Some(Rgba::WHITE);
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("logo"),
            field: F::Color,
            kind: AssetKind::Image,
        },
    );
}

/// The inline kinds are the ones with nothing on disk, and a colour is the
/// second of them. Getting this wrong would demand a `path` for something that
/// is one value in the document.
#[test]
fn a_colour_asset_needs_no_path() {
    let found = problems(&with_a_colour(Rgba::BLACK));
    assert!(
        found.is_empty(),
        "a colour has no file behind it, so nothing should ask for one: {found:?}"
    );
}
