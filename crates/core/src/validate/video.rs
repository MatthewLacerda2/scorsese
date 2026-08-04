//! Generated-video checks: the combinations the provider will not accept.
//!
//! Everything here is answered from the document. The provider's couplings are
//! facts about the request shape, not about the network, so a brief that could
//! never succeed is refused before anything is submitted — which is the
//! difference between a message an agent can act on and a rejection that
//! arrives after the decision to spend was already made.

use crate::asset::{Asset, AssetKind, ClipSeconds, MAX_REFERENCE_IMAGES};
use crate::project::Project;

use super::error::{AssetProblem, VideoProblem};
use super::field::AssetField;

pub(super) fn check(project: &Project, asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(request) = &asset.video else {
        return;
    };
    if asset.kind != AssetKind::GeneratedVideo {
        errors.push(AssetProblem::StrayField {
            asset: asset.id.clone(),
            field: AssetField::Video,
            kind: asset.kind,
        });
        return;
    }

    let mut found = Vec::new();
    if let Some(lock) = request.eight_second_lock()
        && request.seconds != ClipSeconds::Eight
    {
        found.push(VideoProblem::locked(&asset.id, request.seconds, lock));
    }
    if !request.reference_images.is_empty() && !request.model.supports_reference_images() {
        found.push(VideoProblem::unsupported(&asset.id, request.model));
    }
    if request.reference_images.len() > MAX_REFERENCE_IMAGES {
        found.push(VideoProblem::TooManyReferenceImages {
            asset: asset.id.clone(),
            found: request.reference_images.len(),
            max: MAX_REFERENCE_IMAGES,
        });
    }
    if request.last_image.is_some() && request.first_image.is_none() {
        found.push(VideoProblem::LastImageWithoutFirst {
            asset: asset.id.clone(),
        });
    }
    check_stills(project, asset, &mut found);

    errors.extend(found.into_iter().map(Into::into));
}

/// That every still named is a still, and is there to be named.
///
/// The one check here that reads the rest of the table rather than the asset
/// alone — which is the same thing a clip's asset reference is held to, for the
/// same reason: an id pointing at nothing is a rename that did not finish.
fn check_stills(project: &Project, asset: &Asset, found: &mut Vec<VideoProblem>) {
    let request = asset.video_request();
    for referenced in request.images() {
        match project.asset(referenced) {
            None => found.push(VideoProblem::UnknownImage {
                asset: asset.id.clone(),
                referenced: referenced.clone(),
            }),
            Some(still) if still.kind != AssetKind::Image => {
                found.push(VideoProblem::NotAnImage {
                    asset: asset.id.clone(),
                    referenced: referenced.clone(),
                    kind: still.kind,
                });
            }
            Some(_) => {}
        }
    }
}
