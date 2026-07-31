//! Assets-table checks: identity, paths, and kind/lifecycle coherence.

use std::collections::HashSet;

use crate::asset::{Asset, AssetKind};
use crate::project::Project;
use crate::text::{MAX_WEIGHT, MIN_WEIGHT, TextStyle};

use super::error::ValidationError;
use super::field::AssetField;

pub(super) fn check(project: &Project, errors: &mut Vec<ValidationError>) {
    let mut seen = HashSet::new();
    let mut reported = HashSet::new();
    for asset in &project.assets {
        if !seen.insert(&asset.id) && reported.insert(&asset.id) {
            errors.push(ValidationError::DuplicateAssetId {
                id: asset.id.clone(),
            });
        }
        check_path(asset, errors);
        check_recipe(asset, errors);
        check_sha256(asset, errors);
        check_kind_fields(asset, errors);
    }
}

fn check_path(asset: &Asset, errors: &mut Vec<ValidationError>) {
    let Some(path) = &asset.path else {
        return;
    };
    if let Err(problem) = path.check() {
        errors.push(ValidationError::BadPath {
            asset: asset.id.clone(),
            path: path.clone(),
            problem,
        });
    }
}

/// A recipe is a path like any other, so it obeys the same rules — checked
/// here rather than discovered as a missing file part way through a bake.
fn check_recipe(asset: &Asset, errors: &mut Vec<ValidationError>) {
    let Some(recipe) = &asset.recipe else {
        return;
    };
    if let Err(problem) = recipe.check() {
        errors.push(ValidationError::BadRecipePath {
            asset: asset.id.clone(),
            path: recipe.clone(),
            problem,
        });
    }
}

fn check_sha256(asset: &Asset, errors: &mut Vec<ValidationError>) {
    let Some(hash) = &asset.sha256 else {
        return;
    };
    let well_formed = hash.len() == 64 && hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    if !well_formed {
        errors.push(ValidationError::BadSha256 {
            asset: asset.id.clone(),
            value: hash.clone(),
        });
    }
}

/// Which fields an asset must and must not carry follows from its kind: a
/// generated asset needs a brief and a state, a text asset carries its
/// content inline, everything else points at a file.
fn check_kind_fields(asset: &Asset, errors: &mut Vec<ValidationError>) {
    let id = || asset.id.clone();
    let kind = asset.kind;

    check_brief(asset, errors);

    if kind.is_generated() {
        match asset.state {
            None => errors.push(missing(asset, AssetField::State)),
            Some(state) if state.has_media() && asset.path.is_none() => {
                errors.push(ValidationError::GeneratedWithoutPath { asset: id() });
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
}

/// What the inline kinds carry instead of a file.
///
/// `text` holds a string and `color` holds a colour, and each is required by
/// exactly the kind it belongs to and refused everywhere else. Refusing the
/// stray one matters as much as requiring the right one: a `color` on a video
/// asset would never be read, and silence about it would look like it had been.
fn check_inline(asset: &Asset, errors: &mut Vec<ValidationError>) {
    let id = || asset.id.clone();
    let kind = asset.kind;

    match (kind, &asset.text) {
        (AssetKind::Text, None) => errors.push(ValidationError::MissingText { asset: id() }),
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
fn missing(asset: &Asset, field: AssetField) -> ValidationError {
    ValidationError::MissingField {
        asset: asset.id.clone(),
        field,
        kind: asset.kind,
    }
}

/// A field this asset carries that its kind has no use for.
fn stray(asset: &Asset, field: AssetField) -> ValidationError {
    ValidationError::StrayField {
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
fn check_brief(asset: &Asset, errors: &mut Vec<ValidationError>) {
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
fn check_style(asset: &Asset, errors: &mut Vec<ValidationError>) {
    let Some(style) = &asset.style else {
        return;
    };
    if asset.kind != AssetKind::Text {
        errors.push(stray(asset, AssetField::Style));
    }
    if let Some(font) = style.font.file()
        && let Err(problem) = font.check()
    {
        errors.push(ValidationError::BadFontPath {
            asset: asset.id.clone(),
            path: font.clone(),
            problem,
        });
    }
    check_weight(asset, style, errors);
}

/// What can be said about a `weight` without opening a font file.
///
/// Two things, and deliberately only two. A reserved name is a face scorsese
/// ships and knows to be static, so a weight beside one can be refused from the
/// document alone. A number outside OpenType's own `wght` range is not a weight
/// for any face, so it can be refused the same way. Everything else — whether
/// this file is variable at all, and whether its axis reaches 900 — is a fact
/// about bytes on disk, and lives where the file is read. Validation is about
/// the document and stops at the edge of it, exactly as it does for `path`.
fn check_weight(asset: &Asset, style: &TextStyle, errors: &mut Vec<ValidationError>) {
    let Some(weight) = style.weight else {
        return;
    };
    if style.font.is_reserved() {
        errors.push(ValidationError::WeightOnReservedFont {
            asset: asset.id.clone(),
            font: String::from(style.font.clone()),
            weight,
        });
    }
    if !(MIN_WEIGHT..=MAX_WEIGHT).contains(&weight) {
        errors.push(ValidationError::WeightOutOfRange {
            asset: asset.id.clone(),
            weight,
            min: MIN_WEIGHT,
            max: MAX_WEIGHT,
        });
    }
}
