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
use crate::project::Project;
use crate::shape::{Attach, Endpoint, Geometry, MAX_RADIUS};
use crate::timeline::TrackKind;

use super::error::{AssetProblem, ShapeProblem};
use super::field::AssetField;

pub(super) fn check(project: &Project, asset: &Asset, errors: &mut Vec<AssetProblem>) {
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

    match &shape.geometry {
        Geometry::Arrow { from, to, .. } => check_arrow(project, asset, from, to, errors),
        geometry => check_closed(asset, geometry, errors),
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
    if shape.fill.is_some() && !shape.geometry.is_closed() {
        errors.push(ShapeProblem::FilledLine { asset: id() }.into());
    }
    if !shape.draws() {
        errors.push(ShapeProblem::Invisible { asset: id() }.into());
    }
}

/// A rectangle or an ellipse: the questions a shape with an area answers.
fn check_closed(asset: &Asset, geometry: &Geometry, errors: &mut Vec<AssetProblem>) {
    let id = || asset.id.clone();
    // The `None` arm is unreachable — an arrow went the other way at the call
    // site — and answering it as a size of zero rather than unwrapping keeps
    // that true without a panic if the two ever drift apart.
    let (width, height) = geometry.size().unwrap_or((0.0, 0.0));
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
    let radius = geometry.radius();
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
}

/// An arrow: the questions a line answers instead.
///
/// None of them is about where a *point* is on the frame. A point outside it is
/// an arrow entering from off-screen, which is an ordinary thing to draw, so
/// the only refusals about places are a point that is not a pair of numbers at
/// all and two points in the same place — which has no direction, and so no
/// head could be aimed.
///
/// Two ends that are both **attached** are never refused for coinciding, and
/// cannot be: where they land is a fact about a frame, not about the document,
/// and two boxes that happen to sit on top of each other for one shot are not a
/// project that failed to load.
fn check_arrow(
    project: &Project,
    asset: &Asset,
    from: &Endpoint,
    to: &Endpoint,
    errors: &mut Vec<AssetProblem>,
) {
    let id = || asset.id.clone();
    for end in [from, to] {
        if let Some(attach) = end.attach() {
            check_attach(project, asset, attach, errors);
        }
    }
    let (Some(from), Some(to)) = (from.point(), to.point()) else {
        return;
    };
    if !from.is_placed() || !to.is_placed() {
        errors.push(
            ShapeProblem::Unplaced {
                asset: id(),
                from,
                to,
            }
            .into(),
        );
        return;
    }
    if from == to {
        errors.push(
            ShapeProblem::NoLength {
                asset: id(),
                at: from,
            }
            .into(),
        );
    }
}

/// What can be said about an attachment without rendering a frame.
///
/// Three things, and the third is the one that matters most: **an arrow may not
/// attach to a clip that is itself an arrow.** That single rule forecloses
/// every cycle there could be — an arrow following an arrow following the first
/// — without anyone having to walk a graph, and it costs nothing real, because a
/// line has no rectangle worth meeting anyway. It also covers the degenerate
/// case of an arrow attached to a clip showing itself.
fn check_attach(project: &Project, asset: &Asset, attach: &Attach, errors: &mut Vec<AssetProblem>) {
    let id = || asset.id.clone();
    let found = project
        .clips()
        .find(|(_, clip)| clip.id == attach.clip)
        .map(|(track, clip)| (track.kind, clip));
    let Some((kind, clip)) = found else {
        errors.push(
            ShapeProblem::AttachedToNothing {
                asset: id(),
                clip: attach.clip.clone(),
            }
            .into(),
        );
        return;
    };
    if kind != TrackKind::Video {
        errors.push(
            ShapeProblem::AttachedToSound {
                asset: id(),
                clip: attach.clip.clone(),
            }
            .into(),
        );
        return;
    }
    let target_is_a_line = project
        .assets
        .iter()
        .find(|candidate| candidate.id == clip.asset)
        .and_then(|target| target.shape.as_ref())
        .is_some_and(|shape| !shape.geometry.is_closed());
    if target_is_a_line {
        errors.push(
            ShapeProblem::AttachedToArrow {
                asset: id(),
                clip: attach.clip.clone(),
            }
            .into(),
        );
    }
}

/// A measurement that could describe something with area — which rules out
/// zero, negatives, and the two ends of the float range a subtraction can
/// arrive at without anyone having written them down.
fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}
