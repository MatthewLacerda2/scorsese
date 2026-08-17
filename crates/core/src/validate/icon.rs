//! Named-symbol checks: the icons that could never come out as a picture.
//!
//! Two questions, and a third that is deliberately somebody else's.
//!
//! Answered here, from the document alone: whether the `icon` block is on the
//! kind that takes one and off every kind that does not, and whether its two
//! numbers describe something with ink in it.
//!
//! **Not answered here: whether the name is a symbol at all.** The catalogue is
//! a set of property *values* and lives in the compositor, so this crate cannot
//! see it and must not pretend to — the same split a `style`'s font is on, where
//! the path's shape is checked here and whether the face exists is the render's
//! to say. `scorsese check` reports an unknown name beside the missing files, in
//! one list, with the near matches spelled out.

use crate::asset::{Asset, AssetKind};

use super::error::{AssetProblem, IconProblem};
use super::field::AssetField;

pub(super) fn check(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let id = || asset.id.clone();
    let Some(icon) = &asset.icon else {
        if asset.kind == AssetKind::Icon {
            errors.push(AssetProblem::MissingField {
                asset: id(),
                field: AssetField::Icon,
                kind: asset.kind,
            });
        }
        return;
    };
    if asset.kind != AssetKind::Icon {
        errors.push(AssetProblem::StrayField {
            asset: id(),
            field: AssetField::Icon,
            kind: asset.kind,
        });
        return;
    }

    if !positive(icon.size) {
        errors.push(
            IconProblem::NotSized {
                asset: id(),
                size: icon.size,
            }
            .into(),
        );
    }
    if !positive(icon.stroke_width) {
        errors.push(
            IconProblem::NoThickness {
                asset: id(),
                width: icon.stroke_width,
            }
            .into(),
        );
    }
}

/// A measurement that could describe something with ink in it — which rules out
/// zero, negatives, and the two ends of the float range a subtraction can
/// arrive at without anyone having written them down.
fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}
