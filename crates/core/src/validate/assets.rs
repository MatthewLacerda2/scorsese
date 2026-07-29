//! Assets-table checks: identity, paths, and kind/lifecycle coherence.

use std::collections::HashSet;

use crate::asset::{Asset, AssetKind};
use crate::project::Project;

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

    match (kind, &asset.text) {
        (AssetKind::Text, None) => errors.push(ValidationError::MissingText { asset: id() }),
        (AssetKind::Text, Some(_)) => {}
        (_, Some(_)) => errors.push(stray(asset, AssetField::Text)),
        (_, None) => {}
    }
    check_style(asset, errors);
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
}
