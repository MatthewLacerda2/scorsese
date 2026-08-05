//! The brief behind a generated shot: what was asked for, and what asking that
//! way has fixed.
//!
//! The one panel here that edits an **asset** rather than a clip, because that
//! is where a brief lives. Two clips can show the same generated shot, and
//! there is one prompt between them.
//!
//! # Constraints are shown, not discovered
//!
//! This is where most of the value is, and it is a small idea: where a choice
//! fixes another field, the fixed field says so *instead of* accepting a value
//! and failing afterwards. `8s — locked, because 1080p` is a courtesy;
//! `scorsese check` refusing the same document an hour later is the guarantee.
//! Both exist, and the order matters — a rule that fires after somebody has
//! written a brief is a worse experience than a field that was never wrong.
//!
//! **Every one of those rules is asked, never restated.** The lock comes from
//! [`VideoRequest::eight_second_lock`], its reason from [`LengthLock::cause`],
//! and whether a tier takes reference images from
//! [`VideoModel::supports_reference_images`] — the same functions validation
//! uses. A window holding its own copy of the rules would be a second answer,
//! and the first person to find them disagreeing would be somebody who had
//! already paid for a shot.

mod fields;
mod stills;

use egui::{Grid, RichText, Ui};
use scorsese_core::{Asset, AssetId, AssetKind, ClipSeconds, Project, VideoRequest};

use super::selected::Selected;
use super::{Inspector, Refusal};
use crate::project::Open;

/// A generated shot's brief, read out of the document in one pass.
///
/// A snapshot for the reason every other panel here takes one: a control that
/// changes the project cannot also be holding a reference into it.
pub(super) struct Brief {
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

impl Brief {
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
    /// Draws the brief panel under the clip's own fields.
    pub(super) fn brief(
        &mut self,
        ui: &mut Ui,
        open: &mut Open,
        selected: &Selected,
        brief: &Brief,
    ) {
        ui.add_space(10.0);
        ui.label(RichText::new("BRIEF").strong().small());

        if let Some(prompt) = fields::prompt_row(ui, &brief.prompt) {
            self.attempt_brief(open, selected, brief, "the prompt", move |asset| {
                asset.prompt = Some(prompt);
            });
        }

        Grid::new("brief").num_columns(3).show(ui, |ui| {
            if let Some(model) = fields::model_row(ui, brief.request.model) {
                self.attempt_brief(open, selected, brief, "the model", move |asset| {
                    request_of(asset).model = model;
                });
            }
            if let Some(resolution) = fields::resolution_row(ui, brief.request.resolution) {
                self.attempt_brief(open, selected, brief, "the resolution", move |asset| {
                    request_of(asset).resolution = resolution;
                });
            }
            if let Some(seconds) = fields::seconds_row(ui, &brief.request) {
                self.attempt_brief(open, selected, brief, "the length", move |asset| {
                    request_of(asset).seconds = seconds;
                });
            }
            if let Some(aspect) = fields::aspect_row(ui, brief.request.aspect) {
                self.attempt_brief(open, selected, brief, "the aspect", move |asset| {
                    request_of(asset).aspect = aspect;
                });
            }
        });

        stills::show(self, ui, open, selected, brief);
    }

    /// Tries a change to the brief, and remembers why not when it is refused.
    ///
    /// A length the brief has locked is not offered in the first place, so
    /// reaching this with an impossible request takes an outside edit — which
    /// is exactly the case the refusal is for.
    pub(super) fn attempt_brief(
        &mut self,
        open: &mut Open,
        selected: &Selected,
        brief: &Brief,
        what: impl Into<String>,
        change: impl FnOnce(&mut Asset),
    ) {
        // Keyed on the *clip*, like every other refusal here, even though the
        // change was to an asset: what dismisses a refusal is leaving the thing
        // on screen, and what is on screen is a clip.
        self.refused = super::edit::apply_to_asset(open, &brief.asset, change)
            .err()
            .map(|problems| Refusal {
                clip: selected.clip.clone(),
                what: what.into(),
                problems,
            });
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
