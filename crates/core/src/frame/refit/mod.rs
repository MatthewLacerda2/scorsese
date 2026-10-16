//! Changing a project's shape (#1278): carried out, never refused.
//!
//! Most of a layout follows a new shape on its own, because positions, sizes,
//! text and shapes are all fractions of the frame. What does not is footage
//! that **filled** the old frame: a 16:9 shot fitted into a 16:9 frame fills
//! it edge to edge, and the same `fit` in a 9:16 frame is a strip with bars
//! above and below. So such a clip is changed to `fill`, which drew exactly
//! the same pixels in the old frame and fills the new one, cropping what
//! falls outside.
//!
//! What this cannot settle — a title that now wraps past its box, a clip that
//! now leaves the frame — needs a layout to see, and `core` has none: the
//! renderer's checkup reports it against the new frame, in the words #1096
//! gave it, and the agent decides.

use crate::asset::{Asset, AssetKind};
use crate::project::Project;
use crate::timeline::{ClipId, Crop, Fit};

use super::Ratio;

/// What a change of shape did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refit {
    /// The shape the project was.
    pub from: Ratio,
    /// The shape it is now.
    pub to: Ratio,
    /// The clips that filled the old frame and were set to `fill` the new one,
    /// in document order.
    pub filled: Vec<ClipId>,
}

impl Refit {
    /// Whether anything moved at all — false for a project already that shape.
    pub fn changed(&self) -> bool {
        self.from != self.to
    }
}

/// Reshapes `project` to `to`: the shape is set, and every clip whose picture
/// filled the old frame under `fit` is set to `fill` the new one.
///
/// A project already `to` is left exactly as it was. Clips inside groups are
/// weighed too, since a group's members are laid out against the same frame.
pub fn refit(project: &mut Project, to: Ratio) -> Refit {
    let from = project.aspect;
    let mut refit = Refit {
        from,
        to,
        filled: Vec::new(),
    };
    if from == to {
        return refit;
    }
    let shapes: Vec<_> = project
        .assets
        .iter()
        .map(|asset| (asset.id.clone(), picture_shape(asset)))
        .collect();
    for clip in project.every_clip_mut() {
        if clip.fit != Fit::Fit {
            continue;
        }
        let Some(source) = shapes
            .iter()
            .find(|(id, _)| *id == clip.asset)
            .and_then(|(_, shape)| *shape)
        else {
            continue;
        };
        let seen = clip
            .crop
            .as_ref()
            .map_or(Some(source), |crop| cropped(source, crop));
        if seen.is_some_and(|seen| seen.matches(from)) {
            clip.fit = Fit::Fill;
            refit.filled.push(clip.id.clone());
        }
    }
    project.aspect = to;
    refit
}

/// A picture's shape when it is a fact: what a probe measured, or the aspect
/// a generated shot or still was asked for. Inline kinds — a title, a colour,
/// a page — take whatever shape the frame is, so they have none.
pub fn picture_shape(asset: &Asset) -> Option<Ratio> {
    if let Some(media) = &asset.media
        && let (Some(width), Some(height)) = (media.width, media.height)
        && let Some(measured) = Ratio::of_size(width, height)
    {
        return Some(measured);
    }
    let spelled = match asset.kind {
        AssetKind::GeneratedVideo => asset.video_request().aspect.as_str(),
        AssetKind::GeneratedImage => asset.image_request().aspect.as_str(),
        _ => return None,
    };
    spelled.parse().ok()
}

/// The shape of the part of a `source`-shaped picture `crop` keeps.
fn cropped(source: Ratio, crop: &Crop) -> Option<Ratio> {
    // Shapes and fractions of a few thousand parts: exact enough to compare.
    let across = (f64::from(source.across()) * crop.width * 1000.0).round() as u32;
    let down = (f64::from(source.down()) * crop.height * 1000.0).round() as u32;
    Ratio::of_size(across, down)
}

#[cfg(test)]
mod tests;
