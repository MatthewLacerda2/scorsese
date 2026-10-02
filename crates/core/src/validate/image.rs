//! Generated-still checks: the combinations the vendor will not draw.
//!
//! The sibling of [`super::video`], answered from the document for its reason:
//! a brief that could never succeed is refused before anything is sent.

use crate::asset::{Asset, AssetKind, MAX_IMAGE_REFERENCES};
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
    if !request.model.supports(request.size()) {
        found.push(ImageProblem::ResolutionUnsupported {
            asset: asset.id.clone(),
            model: request.model.as_str(),
            resolution: request.size().as_str(),
        });
    }
    if request.reference_images.len() > MAX_IMAGE_REFERENCES {
        found.push(ImageProblem::TooManyReferenceImages {
            asset: asset.id.clone(),
            found: request.reference_images.len(),
            max: MAX_IMAGE_REFERENCES,
        });
    }
    for referenced in &request.reference_images {
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
