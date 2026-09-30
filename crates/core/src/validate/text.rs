//! Text checks beyond the face: the numbers in a `reveal` and a `number` block.
//!
//! Only what the document alone can answer. Whether the face has tabular
//! figures, or a glyph for the separator a locale writes, is a fact about a
//! font file and is the render's to find — the same split the face itself is on.

use crate::asset::Asset;
use crate::text::{MAX_DECIMALS, PLACEHOLDER, TextStyle};

use super::error::{AssetProblem, TextProblem};

pub(super) fn check(asset: &Asset, style: &TextStyle, errors: &mut Vec<AssetProblem>) {
    let id = || asset.id.clone();
    if let Some(reveal) = style.reveal {
        if !(0.0..=1.0).contains(&reveal.stagger) {
            errors.push(
                TextProblem::StaggerOutOfRange {
                    asset: id(),
                    stagger: reveal.stagger,
                }
                .into(),
            );
        }
        if !reveal.rise.is_finite() {
            errors.push(
                TextProblem::RiseNotFinite {
                    asset: id(),
                    rise: reveal.rise,
                }
                .into(),
            );
        }
    }
    let Some(counter) = style.number else {
        return;
    };
    let text = asset.text.as_deref().unwrap_or_default();
    if !text.contains(PLACEHOLDER) {
        errors.push(TextProblem::NoPlaceholder { asset: id() }.into());
    }
    if !counter.value.is_finite() {
        errors.push(
            TextProblem::ValueNotFinite {
                asset: id(),
                value: counter.value,
            }
            .into(),
        );
    }
    if counter.decimals > MAX_DECIMALS {
        errors.push(
            TextProblem::TooManyDecimals {
                asset: id(),
                decimals: counter.decimals,
                max: MAX_DECIMALS,
            }
            .into(),
        );
    }
}
