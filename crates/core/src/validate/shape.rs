//! Drawn-shape checks: the outlines that could never come out as a picture.
//!
//! All of it is answered from the document. A shape carries fractions and two
//! colours and nothing else, so there is no file to open and no raster to know
//! — which makes this the rare check that is complete rather than partial.
//!
//! The one worth understanding is [`ShapeProblem::Invisible`]. Everything else
//! here would produce a frame somebody could look at and see was wrong; a shape
//! with neither fill nor border produces a frame that looks exactly like the
//! shape failing to render, and a diagram quietly missing a box is the kind of
//! mistake that survives all the way into a published video.

use crate::asset::{Asset, AssetKind};
use crate::shape::MAX_RADIUS;

use super::error::{AssetProblem, ShapeProblem};
use super::field::AssetField;

pub(super) fn check(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let id = || asset.id.clone();
    let Some(shape) = &asset.shape else {
        if asset.kind == AssetKind::Shape {
            errors.push(AssetProblem::MissingField {
                asset: id(),
                field: AssetField::Shape,
                kind: asset.kind,
            });
        }
        return;
    };
    if asset.kind != AssetKind::Shape {
        errors.push(AssetProblem::StrayField {
            asset: id(),
            field: AssetField::Shape,
            kind: asset.kind,
        });
        return;
    }

    let (width, height) = (shape.geometry.width(), shape.geometry.height());
    if !positive(width) || !positive(height) {
        errors.push(
            ShapeProblem::NotSized {
                asset: id(),
                width,
                height,
            }
            .into(),
        );
    }
    // `contains` answers false for a NaN radius, which is the answer wanted:
    // not a fraction of anything, so not a rounding this can accept.
    let radius = shape.geometry.radius();
    if !(0.0..=MAX_RADIUS).contains(&radius) {
        errors.push(
            ShapeProblem::RadiusOutOfRange {
                asset: id(),
                radius,
                max: MAX_RADIUS,
            }
            .into(),
        );
    }
    if shape.stroke.is_some() && !positive(shape.stroke_width) {
        errors.push(
            ShapeProblem::BorderWithoutWidth {
                asset: id(),
                width: shape.stroke_width,
            }
            .into(),
        );
    }
    if !shape.draws() {
        errors.push(ShapeProblem::Invisible { asset: id() }.into());
    }
}

/// A measurement that could describe something with area — which rules out
/// zero, negatives, and the two ends of the float range a subtraction can
/// arrive at without anyone having written them down.
fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}
