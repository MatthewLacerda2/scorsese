//! The brief behind a generated line: the words, the voice, the model, and —
//! where the model honours one — the language.
//!
//! The sibling of [`shot`](super::shot), and smaller for the reason the format
//! block is smaller: a shot is a request with couplings in it, and a narration
//! is a sentence, a voice and a price. Four fields, one of which is a
//! constraint rather than a choice.
//!
//! # The language row is the whole argument of this panel
//!
//! [`SpeechModel::Standard`] is the vendor's own default, and it **silently
//! ignores** a language code. So a person who pins `pt` on it gets narration in
//! whatever language the words looked like, having paid for it, with nothing
//! anywhere saying why. The document refuses that combination outright — which
//! is the guarantee — and this panel does not offer it in the first place,
//! which is the courtesy. What appears instead is the reason, naming the model,
//! because the model is the choice a person might reconsider.
//!
//! Switching *to* that model takes a pinned language with it, in the same edit.
//! The alternative is a model change refused because of a field nobody touched,
//! and the precedent is already here: clearing a shot's first still clears its
//! last, for the same reason.

mod fields;
mod voices;

use egui::{Grid, Ui};
use scorsese_core::{Asset, AssetId, AssetKind, Project, SpeechRequest};

use crate::inspector::Inspector;
use crate::inspector::selected::Selected;
use crate::project::Open;

pub(in crate::inspector) use voices::Voices;

/// A generated line's brief, read out of the document in one pass.
///
/// A snapshot for the reason every other panel here takes one: a control that
/// changes the project cannot also be holding a reference into it.
pub(in crate::inspector) struct Line {
    /// The asset the brief belongs to — the handle every edit goes back
    /// through.
    pub(super) asset: AssetId,
    /// The words to be spoken. **The only field that is not optional in the
    /// document**, though a line with no voice chosen is refused at the moment
    /// of spending rather than the moment of writing.
    pub(super) text: String,
    /// Voice, model, language, seed.
    pub(super) request: SpeechRequest,
}

impl Line {
    /// Reads the brief of `asset`, or `None` when it is not a generated line.
    pub(super) fn of(project: &Project, asset: &AssetId) -> Option<Self> {
        let found = project.asset(asset)?;
        if found.kind != AssetKind::GeneratedAudio {
            return None;
        }
        Some(Self {
            asset: asset.clone(),
            text: found.prompt.clone().unwrap_or_default(),
            request: found.speech_request(),
        })
    }
}

impl Inspector {
    /// Draws the line's fields, under the heading [`Inspector::brief`] has
    /// already written.
    pub(super) fn line(&mut self, ui: &mut Ui, open: &mut Open, selected: &Selected, brief: &Line) {
        if let Some(text) = fields::line_row(ui, &brief.text) {
            self.attempt_brief(open, selected, &brief.asset, "the line", move |asset| {
                asset.prompt = Some(text);
            });
        }
        fields::cost_row(ui, &brief.text, brief.request.model);

        // Read once and handed to the row, rather than read inside it: this is
        // a directory of small files, and a panel that read it would read it
        // sixty times a second for as long as a clip stayed selected.
        let known = self.voices.list(&open.root);
        Grid::new("speech").num_columns(3).show(ui, |ui| {
            if let Some(id) = voices::row(ui, brief.request.voice_id.as_deref(), &known) {
                self.attempt_brief(open, selected, &brief.asset, "the voice", move |asset| {
                    request_of(asset).voice_id = Some(id);
                });
            }
            if let Some(model) = fields::model_row(ui, brief.request.model) {
                self.attempt_brief(open, selected, &brief.asset, "the model", move |asset| {
                    let request = request_of(asset);
                    request.model = model;
                    // A language pinned on a model that drops it is a document
                    // the project refuses, so the switch takes it along. The
                    // person who changed the model gets a coherent brief rather
                    // than a refusal about a field they did not touch.
                    if !model.takes_language() {
                        request.language = None;
                    }
                });
            }
            if let Some(language) = fields::language_row(ui, &brief.request) {
                self.attempt_brief(open, selected, &brief.asset, "the language", move |asset| {
                    request_of(asset).language = language;
                });
            }
        });

        // Below the grid rather than in it, because it is a sentence about the
        // whole panel and not a value beside a label.
        if known.is_empty() && voices::nothing_cached(ui) {
            self.voices.forget();
        }
    }
}

/// The asset's request, put there if it had none.
///
/// An asset with no `speech` block means *every default*, not *nothing asked
/// for* — so the first edit to any field of it materialises the block rather
/// than being dropped. That is [`Asset::speech_request`]'s reading, spelled out
/// here because this is the one place that writes it back.
fn request_of(asset: &mut Asset) -> &mut SpeechRequest {
    asset.speech.get_or_insert_with(SpeechRequest::default)
}
