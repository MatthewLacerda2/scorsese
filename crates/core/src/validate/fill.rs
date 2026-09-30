//! Gradient checks, for every field a [`Fill`] can be written in.
//!
//! A solid colour has nothing left to check once it has parsed. A gradient
//! does: the stops have to describe a gradient, and its geometry has to be
//! numbers a frame could be drawn against.

use crate::asset::Asset;
use crate::fill::{Fill, Stop};

use super::error::{AssetProblem, FillProblem};

/// Checks both places a fill is written: a shape's interior and a colour
/// asset. A stray or missing field is not this module's to report — the
/// kind-field checks already do that — so each is checked where it appears.
pub(super) fn check(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    if let Some(fill) = asset.shape.as_ref().and_then(|shape| shape.fill.as_ref()) {
        check_one(asset, "fill", fill, errors);
    }
    if let Some(color) = &asset.color {
        check_one(asset, "color", color, errors);
    }
}

fn check_one(asset: &Asset, field: &'static str, fill: &Fill, errors: &mut Vec<AssetProblem>) {
    let asset_id = || asset.id.clone();
    match fill {
        Fill::Solid(_) => return,
        Fill::Linear(linear) if !linear.angle.is_finite() => {
            errors.push(
                FillProblem::BadAngle {
                    asset: asset_id(),
                    field,
                    angle: linear.angle,
                }
                .into(),
            );
        }
        Fill::Radial(radial)
            if !radial.center.is_placed() || !radial.radius.is_finite() || radial.radius <= 0.0 =>
        {
            errors.push(
                FillProblem::BadCircle {
                    asset: asset_id(),
                    field,
                    x: radial.center.x,
                    y: radial.center.y,
                    radius: radial.radius,
                }
                .into(),
            );
        }
        Fill::Linear(_) | Fill::Radial(_) => {}
    }
    let stops = fill.stops();
    if stops.len() < 2 {
        errors.push(
            FillProblem::TooFewStops {
                asset: asset_id(),
                field,
                count: stops.len(),
            }
            .into(),
        );
    } else if !in_order(stops) {
        errors.push(
            FillProblem::StopsOutOfOrder {
                asset: asset_id(),
                field,
                offsets: stops.iter().map(|stop| stop.at).collect(),
            }
            .into(),
        );
    }
}

/// Every offset inside `0`–`1` and none before the one ahead of it. `contains`
/// answers false for a NaN, which is the answer wanted.
fn in_order(stops: &[Stop]) -> bool {
    stops.iter().all(|stop| (0.0..=1.0).contains(&stop.at))
        && stops.windows(2).all(|pair| pair[0].at <= pair[1].at)
}
