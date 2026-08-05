//! The brief behind a generated shot: a sentence, a tier, a raster, a length,
//! and the stills it is built from.
//!
//! The half of [`super`] that spends the most money per press — ninety-six
//! cents for eight seconds, against about two for a line of narration — which
//! is why so much of it is given over to saying what a choice has already fixed
//! before anybody commits to it. See [`super`] for that rule in full; this
//! module is what it was written about.

mod fields;
mod stills;

use egui::{Grid, Ui};
use scorsese_core::{Asset, AssetId, AssetKind, ClipSeconds, Project, VideoRequest};

use crate::inspector::Inspector;
use crate::inspector::selected::Selected;
use crate::project::Open;

/// A generated shot's brief, read out of the document in one pass.
///
/// A snapshot for the reason every other panel here takes one: a control that
/// changes the project cannot also be holding a reference into it.
pub(in crate::inspector) struct Shot {
    /// The asset the brief belongs to — the handle every edit goes back
    /// through.
    pub(super) asset: AssetId,
    /// The sentence. **The only field that is not optional**, which is why it
    /// is the only one without an `(optional)` beside it.
    pub(super) prompt: String,
    /// Model, resolution, length, aspect, and the stills.
    pub(super) request: VideoRequest,
    /// Every `image` asset in the project, for the pickers. Ids, because that
    /// is what a brief holds and what a document must survive being copied
    /// with.
    pub(super) images: Vec<AssetId>,
}

impl Shot {
    /// Reads the brief of `asset`, or `None` when it is not a generated shot.
    pub(super) fn of(project: &Project, asset: &AssetId) -> Option<Self> {
        let found = project.asset(asset)?;
        if found.kind != AssetKind::GeneratedVideo {
            return None;
        }
        Some(Self {
            asset: asset.clone(),
            prompt: found.prompt.clone().unwrap_or_default(),
            request: found.video_request(),
            images: project
                .assets
                .iter()
                .filter(|asset| asset.kind == AssetKind::Image)
                .map(|asset| asset.id.clone())
                .collect(),
        })
    }
}

impl Inspector {
    /// Draws the shot's fields, under the heading [`super::Inspector::brief`]
    /// has already written.
    pub(super) fn shot(&mut self, ui: &mut Ui, open: &mut Open, selected: &Selected, brief: &Shot) {
        if let Some(prompt) = fields::prompt_row(ui, &brief.prompt) {
            self.attempt_brief(open, selected, &brief.asset, "the prompt", move |asset| {
                asset.prompt = Some(prompt);
            });
        }

        Grid::new("brief").num_columns(3).show(ui, |ui| {
            if let Some(model) = fields::model_row(ui, brief.request.model) {
                self.attempt_brief(open, selected, &brief.asset, "the model", move |asset| {
                    request_of(asset).model = model;
                });
            }
            if let Some(resolution) = fields::resolution_row(ui, brief.request.resolution) {
                self.attempt_brief(
                    open,
                    selected,
                    &brief.asset,
                    "the resolution",
                    move |asset| request_of(asset).resolution = resolution,
                );
            }
            if let Some(seconds) = fields::seconds_row(ui, &brief.request) {
                self.attempt_brief(open, selected, &brief.asset, "the length", move |asset| {
                    request_of(asset).seconds = seconds;
                });
            }
            if let Some(aspect) = fields::aspect_row(ui, brief.request.aspect) {
                self.attempt_brief(open, selected, &brief.asset, "the aspect", move |asset| {
                    request_of(asset).aspect = aspect;
                });
            }
        });

        stills::show(self, ui, open, selected, brief);
    }
}

/// The asset's request, put there if it had none.
///
/// An asset with no `video` block means *every default*, not *nothing asked
/// for* — so the first edit to any field of it materialises the block rather
/// than being dropped. That is [`Asset::video_request`]'s reading, spelled out
/// here because this is the one place that writes it back.
fn request_of(asset: &mut Asset) -> &mut VideoRequest {
    asset.video.get_or_insert_with(VideoRequest::default)
}

/// Whether a length is fixed, and by what — asked of the request rather than
/// worked out here.
pub(super) fn locked_length(request: &VideoRequest) -> Option<(ClipSeconds, &'static str)> {
    request
        .eight_second_lock()
        .map(|lock| (ClipSeconds::Eight, lock.cause()))
}
