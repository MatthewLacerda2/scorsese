//! One value, one field — and where a field is not free, the reason it is not.
//!
//! Each of these reports the new value on the frame it changed and nothing on
//! every other frame, which is the convention every control in this panel
//! follows.

use egui::{ComboBox, RichText, TextEdit, Ui};
use scorsese_core::{Aspect, ClipSeconds, VideoModel, VideoRequest, VideoResolution};

use super::locked_length;

/// What is written beside a field nobody has to fill in.
///
/// Only the prompt is required, so this appears beside everything else — which
/// is the point: a person opening this panel should be able to see in one pass
/// that a sentence is the whole of what a shot needs.
pub(super) const OPTIONAL: &str = "(optional)";

/// The sentence. Multi-line, because a brief is prose and a one-line box makes
/// people write shorter prompts than they meant to.
pub(super) fn prompt_row(ui: &mut Ui, value: &str) -> Option<String> {
    ui.label("Prompt");
    let mut text = value.to_owned();
    let changed = ui
        .add(
            TextEdit::multiline(&mut text)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("what the shot shows"),
        )
        .changed();
    changed.then_some(text)
}

/// The tier, and what choosing the cheaper one gives up.
///
/// **Said before it is chosen, not after.** Whether a tier takes reference
/// images is asked of the model rather than written here, so a tier added to
/// the format cannot quietly acquire the wrong note.
pub(super) fn model_row(ui: &mut Ui, value: VideoModel) -> Option<VideoModel> {
    ui.label("Model");
    let mut chosen = value;
    ComboBox::from_id_salt("brief-model")
        .selected_text(name(value))
        .show_ui(ui, |ui| {
            for option in [VideoModel::Fast, VideoModel::Lite] {
                ui.selectable_value(&mut chosen, option, name(option))
                    .on_hover_text(if option.supports_reference_images() {
                        "Takes reference images"
                    } else {
                        "Cheaper, and takes no reference images"
                    });
            }
        });
    // The third column carries whichever is the more useful thing to say. For
    // a tier that takes reference images that is only that the field is
    // optional; for one that does not, it is the capability being given up —
    // said *before* the choice rather than discovered after it.
    ui.label(
        RichText::new(if value.supports_reference_images() {
            OPTIONAL.to_owned()
        } else {
            format!("{OPTIONAL} · no reference images")
        })
        .weak()
        .small(),
    );
    ui.end_row();
    (chosen != value).then_some(chosen)
}

/// The raster.
pub(super) fn resolution_row(ui: &mut Ui, value: VideoResolution) -> Option<VideoResolution> {
    ui.label("Resolution");
    let mut chosen = value;
    ComboBox::from_id_salt("brief-resolution")
        .selected_text(value.as_str())
        .show_ui(ui, |ui| {
            for option in [VideoResolution::P720, VideoResolution::P1080] {
                ui.selectable_value(&mut chosen, option, option.as_str())
                    .on_hover_text(if option.locks_length() {
                        "Generated at eight seconds only"
                    } else {
                        "Any of the three lengths"
                    });
            }
        });
    ui.label(RichText::new(OPTIONAL).weak().small());
    ui.end_row();
    (chosen != value).then_some(chosen)
}

/// How long the shot is — **or why it is not a choice**.
///
/// The whole idea of this panel in one field. When something about the brief
/// has fixed the length, the other two lengths are not offered and struck
/// through with the reason: `8s — locked, because 1080p`. Offering them and
/// refusing afterwards is the same information delivered too late.
pub(super) fn seconds_row(ui: &mut Ui, request: &VideoRequest) -> Option<ClipSeconds> {
    ui.label("Length");
    if let Some((fixed, cause)) = locked_length(request) {
        ui.label(RichText::new(format!("{}s", fixed.get())).strong());
        ui.label(
            RichText::new(format!("locked, because {cause}"))
                .weak()
                .small(),
        );
        ui.end_row();
        return None;
    }

    let value = request.seconds;
    let mut chosen = value;
    ComboBox::from_id_salt("brief-seconds")
        .selected_text(format!("{}s", value.get()))
        .show_ui(ui, |ui| {
            for option in ClipSeconds::ALL {
                ui.selectable_value(&mut chosen, option, format!("{}s", option.get()));
            }
        });
    ui.label(RichText::new(OPTIONAL).weak().small());
    ui.end_row();
    (chosen != value).then_some(chosen)
}

/// Landscape or upright.
pub(super) fn aspect_row(ui: &mut Ui, value: Aspect) -> Option<Aspect> {
    ui.label("Aspect");
    let mut chosen = value;
    ComboBox::from_id_salt("brief-aspect")
        .selected_text(aspect_name(value))
        .show_ui(ui, |ui| {
            for option in [Aspect::Wide, Aspect::Tall] {
                ui.selectable_value(&mut chosen, option, aspect_name(option));
            }
        });
    ui.label(RichText::new(OPTIONAL).weak().small());
    ui.end_row();
    (chosen != value).then_some(chosen)
}

/// What a tier is called on screen.
///
/// Exhaustive on purpose: a tier added to the format fails to compile here,
/// which is the cheapest possible reminder that the window has to name it too.
fn name(model: VideoModel) -> &'static str {
    match model {
        VideoModel::Fast => "Fast",
        VideoModel::Lite => "Lite",
    }
}

/// What an aspect is called on screen — the ratio, because that is what a
/// person picking one is thinking in.
fn aspect_name(aspect: Aspect) -> &'static str {
    match aspect {
        Aspect::Wide => "16:9",
        Aspect::Tall => "9:16",
    }
}
