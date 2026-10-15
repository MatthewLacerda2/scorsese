//! The brief behind a generated still: a sentence, a size, a shape, and the
//! pictures it is drawn from.
//!
//! The third brief, beside [`shot`](super::shot) and [`line`](super::line),
//! and kept apart from both for [`super`]'s reason. There is no model to pick:
//! every still is Nano Banana 2.1's (#1308), which draws every shape at every
//! size, so both drop-downs offer the whole list.
//!
//! **References are listed, not picked.** Which pictures a still is drawn
//! from is a structured choice — a character sheet, two props — and the rule
//! for the window is that anything with structure to it is a sentence to an
//! assistant. So the panel says what they are, by kind, and leaves choosing
//! them to that.

use egui::{ComboBox, Grid, RichText, TextEdit, Ui};
use scorsese_core::{
    Asset, AssetId, AssetKind, ImageAspect, ImageRequest, ImageResolution, Project, ReferenceKind,
};

use crate::inspector::Inspector;
use crate::inspector::selected::Selected;
use crate::project::Open;

/// A generated still's brief, read out of the document in one pass.
pub(in crate::inspector) struct Picture {
    /// The asset the brief belongs to.
    asset: AssetId,
    /// The sentence.
    prompt: String,
    /// Size, aspect and references.
    request: ImageRequest,
}

impl Picture {
    /// Reads the brief of `asset`, or `None` when it is not a generated still.
    pub(super) fn of(project: &Project, asset: &AssetId) -> Option<Self> {
        let found = project.asset(asset)?;
        (found.kind == AssetKind::GeneratedImage).then(|| Self {
            asset: asset.clone(),
            prompt: found.prompt.clone().unwrap_or_default(),
            request: found.image_request(),
        })
    }
}

impl Inspector {
    /// Draws the still's fields, under the heading [`Inspector::brief`] wrote.
    pub(super) fn picture(
        &mut self,
        ui: &mut Ui,
        open: &mut Open,
        selected: &Selected,
        brief: &Picture,
    ) {
        ui.label("Prompt");
        let mut prompt = brief.prompt.clone();
        let edited = ui
            .add(
                TextEdit::multiline(&mut prompt)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("what the picture shows"),
            )
            .changed();
        if edited {
            self.attempt_brief(open, selected, &brief.asset, "the prompt", move |asset| {
                asset.prompt = Some(prompt);
            });
        }

        let request = &brief.request;
        Grid::new("picture-brief").num_columns(2).show(ui, |ui| {
            if let Some(size) = choose(
                ui,
                "Size",
                request.size(),
                &ImageResolution::ALL,
                ImageResolution::as_str,
            ) {
                self.attempt_brief(open, selected, &brief.asset, "the size", move |asset| {
                    request_of(asset).resolution = Some(size);
                });
            }
            if let Some(aspect) = choose(
                ui,
                "Aspect",
                request.aspect,
                &ImageAspect::ALL,
                ImageAspect::as_str,
            ) {
                self.attempt_brief(open, selected, &brief.asset, "the aspect", move |asset| {
                    request_of(asset).aspect = aspect;
                });
            }
        });

        let mut said: Vec<String> = ReferenceKind::ALL
            .into_iter()
            .filter_map(|kind| {
                let named = request.references_of(kind);
                let names: Vec<&str> = named.iter().map(AssetId::as_str).collect();
                (!names.is_empty()).then(|| format!("{}: {}", kind_word(kind), names.join(", ")))
            })
            .collect();
        if said.is_empty() {
            said.push(String::from("No reference images"));
        }
        for line in said {
            ui.label(RichText::new(line).weak().small());
        }
    }
}

/// What a kind of reference is called in the panel.
fn kind_word(kind: ReferenceKind) -> &'static str {
    match kind {
        ReferenceKind::Object => "Objects",
        ReferenceKind::Character => "Characters",
    }
}

/// One drop-down: the new value on the frame it changed, nothing otherwise.
fn choose<T: Copy + PartialEq>(
    ui: &mut Ui,
    label: &str,
    value: T,
    options: &[T],
    name: fn(T) -> &'static str,
) -> Option<T> {
    ui.label(label);
    let mut chosen = value;
    ComboBox::from_id_salt(("picture-brief", label))
        .selected_text(name(value))
        .show_ui(ui, |ui| {
            for option in options {
                ui.selectable_value(&mut chosen, *option, name(*option));
            }
        });
    ui.end_row();
    (chosen != value).then_some(chosen)
}

/// The asset's request, put there if it had none — absent means every default,
/// so the first edit materialises the block rather than being dropped.
fn request_of(asset: &mut Asset) -> &mut ImageRequest {
    asset.image.get_or_insert_with(ImageRequest::default)
}
