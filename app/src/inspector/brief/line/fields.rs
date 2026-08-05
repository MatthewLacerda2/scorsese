//! One value, one field — and where a field is not a choice at all, the reason
//! it is not.
//!
//! The speech half of [`shot::fields`](crate::inspector::brief::shot), and it
//! follows the same convention: each of these reports the new value on the
//! frame it changed and nothing on every other frame.

use egui::{ComboBox, RichText, TextEdit, Ui};
use scorsese_core::{MAX_CHARACTERS, SpeechModel, SpeechRequest};
use scorsese_providers::prices::{dollars, speech};

/// What is written beside a field nobody has to fill in.
pub(super) const OPTIONAL: &str = "(optional)";

/// How many characters the rate table is quoted per, which is also how a
/// vendor's price list writes it.
const PER: usize = 1_000;

/// The words. Multi-line, because narration is prose and a one-line box makes
/// people write shorter lines than they meant to.
pub(super) fn line_row(ui: &mut Ui, value: &str) -> Option<String> {
    ui.label("Line");
    let mut text = value.to_owned();
    let changed = ui
        .add(
            TextEdit::multiline(&mut text)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("what the voice says"),
        )
        .changed();
    changed.then_some(text)
}

/// What this line is billed by, and what that comes to.
///
/// Under the words rather than in the grid, because it is *about* them: speech
/// is billed per character, so the length of the line is the price, and seeing
/// the two together is the point of putting a brief on screen at all. The
/// arithmetic is asked of [`speech`] — the same call the ceiling is checked
/// against and the same one the run records — so a figure here can never
/// disagree with the one somebody is charged for.
///
/// Characters and not bytes, which is the vendor's unit and the only one that
/// prices Portuguese correctly.
pub(super) fn cost_row(ui: &mut Ui, text: &str, model: SpeechModel) {
    let characters = text.chars().count();
    // Said before it is refused, like every other constraint in this panel.
    // Validation reports the same thing, and a person who has just pasted a
    // chapter should not have to run it to find out.
    if characters > MAX_CHARACTERS {
        ui.label(
            RichText::new(format!(
                "{characters} characters — {MAX_CHARACTERS} is the most one request speaks"
            ))
            .color(ui.visuals().warn_fg_color)
            .small(),
        );
        return;
    }
    let cents = speech(model, characters).map_or(0, |estimate| estimate.cents);
    ui.label(
        RichText::new(format!(
            "{characters} characters · about {}, by our own calculation",
            dollars(cents)
        ))
        .weak()
        .small(),
    );
}

/// Which model reads the line, and what each one costs.
///
/// The rate in the hover is asked of the price table rather than written here,
/// so a table edited without this file cannot leave a stale number on screen.
pub(super) fn model_row(ui: &mut Ui, value: SpeechModel) -> Option<SpeechModel> {
    ui.label("Model");
    let mut chosen = value;
    ComboBox::from_id_salt("brief-speech-model")
        .selected_text(name(value))
        .show_ui(ui, |ui| {
            for option in SpeechModel::ALL {
                ui.selectable_value(&mut chosen, option, name(option))
                    .on_hover_text(format!("{} · {}", about(option), rate(option)));
            }
        });
    // The third column carries whichever is the more useful thing to say. For a
    // model that honours a language that is only that the field is optional;
    // for the one that does not, it is the capability being given up — said
    // *before* the choice rather than discovered after it.
    ui.label(
        RichText::new(if value.takes_language() {
            OPTIONAL.to_owned()
        } else {
            format!("{OPTIONAL} · ignores a language")
        })
        .weak()
        .small(),
    );
    ui.end_row();
    (chosen != value).then_some(chosen)
}

/// The language to pin the reading to — **or why it is not a choice**.
///
/// The whole idea of this panel in one field. On the model that drops the
/// field, the box is not offered and the reason is given instead, naming the
/// model: the vendor accepts a language there and ignores it, so a narration
/// would arrive in the wrong one having been paid for, with no error anywhere
/// to read. Offering the field and refusing on save is the same information
/// delivered too late; offering it and *succeeding* would be worse still.
///
/// A plain box rather than a list of languages, because scorsese publishes no
/// list of them: the vendor takes an ISO 639-1 code, and a window inventing its
/// own dozen would be the window deciding which languages this editor is for.
pub(super) fn language_row(ui: &mut Ui, request: &SpeechRequest) -> Option<Option<String>> {
    ui.label("Language");
    if !request.model.takes_language() {
        ui.label(RichText::new("the model decides").strong());
        ui.label(
            RichText::new(format!(
                "not a choice, because {} silently ignores it",
                name(request.model)
            ))
            .weak()
            .small(),
        );
        ui.end_row();
        return None;
    }

    let value = request.language.clone().unwrap_or_default();
    let mut code = value.clone();
    let changed = ui
        .add(
            TextEdit::singleline(&mut code)
                .desired_width(60.0)
                .hint_text("en"),
        )
        .changed();
    ui.label(
        RichText::new(format!("{OPTIONAL} · ISO 639-1 — en, pt"))
            .weak()
            .small(),
    );
    ui.end_row();
    // An emptied box means *let the model decide*, which is the absent field
    // rather than a blank one — a blank code is a document that says it pinned
    // a language and did not.
    changed.then(|| (!code.trim().is_empty()).then_some(code))
}

/// What a model is called on screen.
///
/// Exhaustive on purpose: a model added to the format fails to compile here,
/// which is the cheapest possible reminder that the window has to name it too.
fn name(model: SpeechModel) -> &'static str {
    match model {
        SpeechModel::Expressive => "Expressive",
        SpeechModel::Standard => "Standard",
        SpeechModel::Fast => "Fast",
    }
}

/// What choosing a model is actually a choice about.
fn about(model: SpeechModel) -> &'static str {
    match model {
        SpeechModel::Expressive => "The most expressive reading",
        SpeechModel::Standard => "The vendor's default, and it ignores a pinned language",
        SpeechModel::Fast => "Quickest, and half the price",
    }
}

/// What a model costs, read out of the price table.
fn rate(model: SpeechModel) -> String {
    match speech(model, PER) {
        Ok(estimate) => format!("about {} per {PER} characters", dollars(estimate.cents)),
        // A model with no published rate cannot be quoted, and saying so is the
        // honest answer — a blank where a price belongs reads as free.
        Err(why) => why.to_string(),
    }
}
