//! The pictures a brief is built from, picked out of the project's own pool.
//!
//! **Never a file browser**, and that is a format rule rather than a taste:
//! every path in a `project.json` is relative to the project root, because a
//! project directory has to survive `scp -r` to another machine. A file picker
//! puts an absolute path into a document, and the document is wrong the moment
//! it is copied. So these are pickers over the `image` assets already imported,
//! and getting a picture in is `scorsese import` or a drop onto the pool —
//! which is the same route every other file takes.

use egui::{ComboBox, RichText, Ui};
use scorsese_core::{AssetId, MAX_REFERENCE_IMAGES, VideoModel};

use super::fields::OPTIONAL;
use super::{Shot, request_of};
use crate::inspector::Inspector;
use crate::inspector::selected::Selected;
use crate::project::Open;

/// What the empty entry in a picker reads as.
const NONE: &str = "— none —";

/// Draws the three still fields.
pub(super) fn show(
    inspector: &mut Inspector,
    ui: &mut Ui,
    open: &mut Open,
    selected: &Selected,
    brief: &Shot,
) {
    if brief.images.is_empty() {
        ui.add_space(4.0);
        ui.label(
            RichText::new("No images in this project to build a shot from — import one first.")
                .weak()
                .small(),
        );
        return;
    }

    ui.add_space(6.0);
    if let Some(picked) = one(ui, "First image", &brief.request.first_image, &brief.images) {
        inspector.attempt_brief(open, selected, &brief.asset, "the first image", move |asset| {
            let request = request_of(asset);
            request.first_image = picked;
            // A last frame with no first is not a request the provider has —
            // it is half an interpolation. Clearing it here means the person
            // who removed the first image gets a coherent brief rather than a
            // refusal about a field they did not touch.
            if request.first_image.is_none() {
                request.last_image = None;
            }
        });
    }

    // Offered only once there is a first image, because that is the only
    // arrangement that means anything: these two are the ends of one
    // interpolation, and a last frame alone has nothing to interpolate from.
    if brief.request.first_image.is_some() {
        if let Some(picked) = one(ui, "Last image", &brief.request.last_image, &brief.images) {
            inspector.attempt_brief(open, selected, &brief.asset, "the last image", move |asset| {
                request_of(asset).last_image = picked;
            });
        }
    } else {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Last image").weak());
            ui.label(RichText::new("needs a first image").weak().small());
        });
    }

    references(inspector, ui, open, selected, brief);
}

/// The stills that say what a subject looks like.
///
/// Both limits are shown rather than discovered: a tier that does not take them
/// says so instead of offering a picker, and a full list says it is full rather
/// than accepting a fourth and being refused.
fn references(
    inspector: &mut Inspector,
    ui: &mut Ui,
    open: &mut Open,
    selected: &Selected,
    brief: &Shot,
) {
    ui.add_space(6.0);
    let chosen = &brief.request.reference_images;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Reference images").strong().small());
        ui.label(RichText::new(OPTIONAL).weak().small());
    });

    if !brief.request.model.supports_reference_images() {
        ui.label(
            RichText::new(format!(
                "not taken by {}",
                match brief.request.model {
                    VideoModel::Lite => "Lite",
                    VideoModel::Fast => "this model",
                }
            ))
            .weak()
            .small(),
        );
        return;
    }

    for (index, id) in chosen.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(id.as_str()).small());
            if ui.small_button("✕").on_hover_text("Remove").clicked() {
                inspector.attempt_brief(open, selected, &brief.asset, "a reference image", move |asset| {
                    request_of(asset).reference_images.remove(index);
                });
            }
        });
    }

    if chosen.len() >= MAX_REFERENCE_IMAGES {
        ui.label(
            RichText::new(format!("{MAX_REFERENCE_IMAGES} is the most a shot takes"))
                .weak()
                .small(),
        );
        return;
    }

    // The pool minus what is already on the brief: naming the same still twice
    // says nothing the once did not, and offering it invites the question.
    let free: Vec<AssetId> = brief
        .images
        .iter()
        .filter(|id| !chosen.contains(id))
        .cloned()
        .collect();
    if free.is_empty() {
        return;
    }
    let mut adding: Option<AssetId> = None;
    ComboBox::from_id_salt("brief-add-reference")
        .selected_text("Add…")
        .show_ui(ui, |ui| {
            for id in &free {
                if ui.selectable_label(false, id.as_str()).clicked() {
                    adding = Some(id.clone());
                }
            }
        });
    if let Some(id) = adding {
        inspector.attempt_brief(open, selected, &brief.asset, "a reference image", move |asset| {
            request_of(asset).reference_images.push(id);
        });
    }
}

/// One optional still: the pool, plus the entry that means none of it.
fn one(
    ui: &mut Ui,
    label: &str,
    value: &Option<AssetId>,
    images: &[AssetId],
) -> Option<Option<AssetId>> {
    let mut chosen = value.clone();
    ui.horizontal(|ui| {
        ui.label(label);
        ComboBox::from_id_salt(label)
            .selected_text(value.as_ref().map_or(NONE, AssetId::as_str))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut chosen, None, NONE);
                for id in images {
                    ui.selectable_value(&mut chosen, Some(id.clone()), id.as_str());
                }
            });
        ui.label(RichText::new(OPTIONAL).weak().small());
    });
    (chosen != *value).then_some(chosen)
}
