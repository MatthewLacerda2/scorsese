//! Assets-table checks: identity, paths, and kind/lifecycle coherence.

use std::collections::HashSet;

use crate::asset::{Asset, AssetKind};
use crate::project::Project;

use super::error::ValidationError;

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
/// prompt-backed asset needs a prompt and a state, a text asset carries its
/// content inline, everything else points at a file.
fn check_kind_fields(asset: &Asset, errors: &mut Vec<ValidationError>) {
    let id = || asset.id.clone();
    let kind = asset.kind;

    if kind.is_generated() {
        if asset.prompt.is_none() {
            errors.push(ValidationError::MissingPrompt { asset: id(), kind });
        }
        match asset.state {
            None => errors.push(ValidationError::MissingState { asset: id(), kind }),
            Some(state) if state.has_media() && asset.path.is_none() => {
                errors.push(ValidationError::GeneratedWithoutPath { asset: id() });
            }
            Some(_) => {}
        }
    } else {
        if asset.prompt.is_some() {
            errors.push(ValidationError::PromptOnPlainAsset { asset: id(), kind });
        }
        if asset.state.is_some() {
            errors.push(ValidationError::StateOnPlainAsset { asset: id(), kind });
        }
        if kind.is_file_backed() && asset.path.is_none() {
            errors.push(ValidationError::MissingPath { asset: id(), kind });
        }
    }

    match (kind, &asset.text) {
        (AssetKind::Text, None) => errors.push(ValidationError::MissingText { asset: id() }),
        (kind, Some(_)) if kind != AssetKind::Text => {
            errors.push(ValidationError::TextOnNonTextAsset { asset: id(), kind });
        }
        _ => {}
    }
}
