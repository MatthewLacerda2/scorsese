//! Assets-table checks: identity, paths, and kind/lifecycle coherence.

use std::collections::HashSet;

use crate::asset::{Asset, AssetKind};
use crate::project::Project;
use crate::text::{MAX_WEIGHT, MIN_WEIGHT, TextStyle};

use super::error::AssetProblem;
use super::field::AssetField;

pub(super) fn check(project: &Project) -> Vec<AssetProblem> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    let mut reported = HashSet::new();
    for asset in &project.assets {
        if !seen.insert(&asset.id) && reported.insert(&asset.id) {
            errors.push(AssetProblem::DuplicateAssetId {
                id: asset.id.clone(),
            });
        }
        check_path(asset, &mut errors);
        check_recipe(asset, &mut errors);
        check_sha256(asset, &mut errors);
        check_kind_fields(asset, &mut errors);
        super::shape::check(asset, &mut errors);
        super::video::check(project, asset, &mut errors);
        super::speech::check(asset, &mut errors);
    }
    errors
}

fn check_path(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(path) = &asset.path else {
        return;
    };
    if let Err(problem) = path.check() {
        errors.push(AssetProblem::BadPath {
            asset: asset.id.clone(),
            path: path.clone(),
            problem,
        });
    }
}

/// A recipe is a path like any other, so it obeys the same rules — checked
/// here rather than discovered as a missing file part way through a bake.
fn check_recipe(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(recipe) = &asset.recipe else {
        return;
    };
    if let Err(problem) = recipe.check() {
        errors.push(AssetProblem::BadRecipePath {
            asset: asset.id.clone(),
            path: recipe.clone(),
            problem,
        });
    }
}

fn check_sha256(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(hash) = &asset.sha256 else {
        return;
    };
    let well_formed = hash.len() == 64 && hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    if !well_formed {
        errors.push(AssetProblem::BadSha256 {
            asset: asset.id.clone(),
            value: hash.clone(),
        });
    }
}

/// Which fields an asset must and must not carry follows from its kind: a
/// generated asset needs a brief and a state, a text asset carries its
/// content inline, everything else points at a file.
fn check_kind_fields(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let id = || asset.id.clone();
    let kind = asset.kind;

    check_brief(asset, errors);

    if kind.is_generated() {
        match asset.state {
            None => errors.push(missing(asset, AssetField::State)),
            Some(state) if state.has_media() && asset.path.is_none() => {
                errors.push(AssetProblem::GeneratedWithoutPath { asset: id() });
            }
            Some(_) => {}
        }
    } else {
        if asset.state.is_some() {
            errors.push(stray(asset, AssetField::State));
        }
        if kind.is_file_backed() && asset.path.is_none() {
            errors.push(missing(asset, AssetField::Path));
        }
    }

    check_inline(asset, errors);
    check_style(asset, errors);
    check_bookkeeping(asset, errors);
}

/// What a kind can have a record *of*.
///
/// These are not briefs and nothing renders from them, but they are refused off
/// the kinds that cannot produce them all the same: a ticket on an imported
/// file names work nobody is doing, and a price on a synthesised one is money
/// that was never spent. A number nothing could have written is worth a
/// question, even when nothing would have read it.
fn check_bookkeeping(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let kind = asset.kind;
    if asset.operation.is_some() && kind != AssetKind::GeneratedVideo {
        errors.push(stray(asset, AssetField::Operation));
    }
    if asset.queued_at.is_some() && !kind.is_generated() {
        errors.push(stray(asset, AssetField::QueuedAt));
    }
    if asset.estimated_cost_cents.is_some() && !kind.is_prompted() {
        errors.push(stray(asset, AssetField::Cost));
    }
}

/// What the inline kinds carry instead of a file.
///
/// `text` holds a string and `color` holds a colour, and each is required by
/// exactly the kind it belongs to and refused everywhere else. Refusing the
/// stray one matters as much as requiring the right one: a `color` on a video
/// asset would never be read, and silence about it would look like it had been.
fn check_inline(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let id = || asset.id.clone();
    let kind = asset.kind;

    match (kind, &asset.text) {
        (AssetKind::Text, None) => errors.push(AssetProblem::MissingText { asset: id() }),
        (AssetKind::Text, Some(_)) => {}
        (_, Some(_)) => errors.push(stray(asset, AssetField::Text)),
        (_, None) => {}
    }
    match (kind, asset.color) {
        (AssetKind::Color, None) => errors.push(missing(asset, AssetField::Color)),
        (AssetKind::Color, Some(_)) => {}
        (_, Some(_)) => errors.push(stray(asset, AssetField::Color)),
        (_, None) => {}
    }
}

/// A field this asset's kind requires and does not have.
fn missing(asset: &Asset, field: AssetField) -> AssetProblem {
    AssetProblem::MissingField {
        asset: asset.id.clone(),
        field,
        kind: asset.kind,
    }
}

/// A field this asset carries that its kind has no use for.
fn stray(asset: &Asset, field: AssetField) -> AssetProblem {
    AssetProblem::StrayField {
        asset: asset.id.clone(),
        field,
        kind: asset.kind,
    }
}

/// The brief: what an asset is to be made *from*.
///
/// Two forms, one per kind that takes one — a `prompt` is a sentence handed to
/// a provider, a `recipe` is a document synthesised locally — and each is
/// required by exactly the kinds it belongs to and refused everywhere else.
/// Refusing the stray one matters as much as requiring the right one: a recipe
/// on a Veo asset would never be read, and silence about it would look like it
/// had been.
fn check_brief(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let kind = asset.kind;

    match (kind.is_prompted(), asset.prompt.is_some()) {
        (true, false) => errors.push(missing(asset, AssetField::Prompt)),
        (false, true) => errors.push(stray(asset, AssetField::Prompt)),
        _ => {}
    }
    match (kind.is_synthesized(), asset.recipe.is_some()) {
        (true, false) => errors.push(missing(asset, AssetField::Recipe)),
        (false, true) => errors.push(stray(asset, AssetField::Recipe)),
        _ => {}
    }
}

/// A style belongs to the one kind that has glyphs, and the font it names is a
/// path like any other — so it obeys the same rules, checked here rather than
/// discovered as a missing file part way through a render.
fn check_style(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(style) = &asset.style else {
        return;
    };
    if asset.kind != AssetKind::Text {
        errors.push(stray(asset, AssetField::Style));
    }
    if let Some(font) = style.font.file()
        && let Err(problem) = font.check()
    {
        errors.push(AssetProblem::BadFontPath {
            asset: asset.id.clone(),
            path: font.clone(),
            problem,
        });
    }
    check_weight(asset, style, errors);
}

/// What can be said about a `weight` without opening a font file.
///
/// One thing, and deliberately only one: a number outside OpenType's own `wght`
/// range is not a weight for any face. Everything else — whether this file is
/// variable at all, and whether its axis reaches 900 — is a fact about bytes,
/// and lives where the file is read. Validation is about the document and stops
/// at the edge of it, exactly as it does for `path`.
///
/// A weight beside `sans` or `serif` used to be refused here, on the grounds
/// that the shipped faces were one static weight each. They are variable now,
/// so the refusal went with the reason for it. Whether Source Serif 4's axis
/// reaches the 100 that Inter's does is a fact about a file like any other, and
/// is refused at the render — from where the answer actually is.
fn check_weight(asset: &Asset, style: &TextStyle, errors: &mut Vec<AssetProblem>) {
    let Some(weight) = style.weight else {
        return;
    };
    if !(MIN_WEIGHT..=MAX_WEIGHT).contains(&weight) {
        errors.push(AssetProblem::WeightOutOfRange {
            asset: asset.id.clone(),
            weight,
            min: MIN_WEIGHT,
            max: MAX_WEIGHT,
        });
    }
}
