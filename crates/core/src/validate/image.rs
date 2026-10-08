//! Generated-still checks: the combinations the vendor will not draw.
//!
//! The sibling of [`super::video`], answered from the document for its reason:
//! a brief that could never succeed is refused before anything is sent.

use crate::asset::{Asset, AssetKind, ImageModel, ImageResolution, ReferenceKind};
use crate::project::Project;

use super::error::{AssetProblem, ImageProblem};
use super::field::AssetField;

pub(super) fn check(project: &Project, asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(request) = &asset.image else {
        return;
    };
    if asset.kind != AssetKind::GeneratedImage {
        errors.push(AssetProblem::StrayField {
            asset: asset.id.clone(),
            field: AssetField::Image,
            kind: asset.kind,
        });
        return;
    }

    let mut found = Vec::new();
    let model = request.model;
    if !model.supports(request.size()) {
        found.push(ImageProblem::ResolutionUnsupported {
            asset: asset.id.clone(),
            model: model.as_str(),
            resolution: request.size().as_str(),
            draws: sizes_of(model),
        });
    }
    if !model.draws(request.aspect) {
        found.push(ImageProblem::AspectUnsupported {
            asset: asset.id.clone(),
            model: model.as_str(),
            aspect: request.aspect.as_str(),
        });
    }
    if let Some(thinking) = request.thinking
        && !model.thinking_levels().contains(&thinking)
    {
        found.push(ImageProblem::ThinkingUnsupported {
            asset: asset.id.clone(),
            model: model.as_str(),
            thinking: thinking.as_str(),
            offers: levels_of(model),
        });
    }
    for kind in ReferenceKind::ALL {
        let named = request.references_of(kind).len();
        let max = model.references(kind);
        if named > max {
            found.push(ImageProblem::TooManyReferenceImages {
                asset: asset.id.clone(),
                model: model.as_str(),
                field: kind.field(),
                found: named,
                max,
            });
        }
    }
    for referenced in request.references() {
        let problem = match project.asset(referenced) {
            None => ImageProblem::UnknownImage {
                asset: asset.id.clone(),
                referenced: referenced.clone(),
            },
            Some(still) if still.id == asset.id => ImageProblem::ReferencesItself {
                asset: asset.id.clone(),
            },
            Some(still) if !still.kind.is_still() => ImageProblem::NotAnImage {
                asset: asset.id.clone(),
                referenced: referenced.clone(),
                kind: still.kind,
            },
            Some(_) => continue,
        };
        found.push(problem);
    }

    errors.extend(found.into_iter().map(Into::into));
}

/// The sizes a model draws, as a message lists them: `1K, 2K or 4K`.
fn sizes_of(model: ImageModel) -> String {
    let sizes: Vec<_> = ImageResolution::ALL
        .into_iter()
        .filter(|size| model.supports(*size))
        .map(ImageResolution::as_str)
        .collect();
    listed(&sizes)
}

/// The thinking levels a model offers, as a message lists them.
fn levels_of(model: ImageModel) -> String {
    let levels: Vec<_> = model.thinking_levels().iter().map(|l| l.as_str()).collect();
    if levels.is_empty() {
        String::from("no level to choose")
    } else {
        listed(&levels)
    }
}

/// `a`, `a or b`, `a, b or c`.
fn listed(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}
