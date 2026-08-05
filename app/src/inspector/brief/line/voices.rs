//! The voice, picked from a list rather than typed as an id.
//!
//! **Never a text box wanting a hexadecimal id**, and that is not a taste: a
//! `voice_id` is the vendor's, it is not guessable, and pasting one out of a
//! browser is how a project ends up naming a voice that no longer exists.
//! Every ElevenLabs default voice expires on 2026-12-31, so scorsese ships no
//! default and holds no list — [`voices`](scorsese_providers::voices) resolves
//! one at runtime and keeps it under the project's `cache/`.
//!
//! # The window reads that cache and never fetches
//!
//! Fetching needs a key, a network and a permission this repository's own test
//! key does not have, and any of the three can be missing while somebody is
//! editing. A panel that fetched would therefore have a spinner, a failure
//! state and an error taxonomy in it — for a dropdown. So it reads what
//! `scorsese voices` and the `voices` MCP tool have already put there, and when
//! that is nothing it says so and says which command fills it. Free, instant,
//! and impossible to make block.

use std::path::Path;
use std::rc::Rc;

use egui::{ComboBox, RichText, Ui};
use scorsese_providers::voices::{Voice, cached};

/// What the picker reads as before anybody has chosen.
///
/// Not a voice. There is no default to pre-select and there cannot be one, so
/// the empty state is stated rather than filled in with something plausible.
const NONE: &str = "— none chosen —";

/// The cached voice list, as the inspector remembers it.
///
/// Read once per project and kept, because the alternative is a directory
/// listing and a handful of JSON parses on every repaint — sixty times a second
/// for as long as a narration clip stays selected. What that costs is freshness
/// while the window is open, and [`Voices::forget`] is what buys it back. Two
/// things ask for it: [`crate::project::watch`], which notices `scorsese
/// voices` filling the cache in the next terminal along, and the button beside
/// the empty list — kept, because a watch that misses a look leaves somebody
/// needing a way to insist.
#[derive(Debug, Default)]
pub(in crate::inspector) struct Voices {
    /// What the last read found. `None` means *not read yet*, which is not the
    /// same as *read, and there were none* — the second one is a message the
    /// panel has to be able to show.
    known: Option<Rc<[Voice]>>,
}

impl Voices {
    /// The voices this project has cached, reading them if this is the first
    /// ask.
    ///
    /// An [`Rc`] rather than a slice, so a caller can hold the list while
    /// making an edit through the same `&mut self` it came from.
    pub(in crate::inspector) fn list(&mut self, root: &Path) -> Rc<[Voice]> {
        self.known
            .get_or_insert_with(|| cached(root).into())
            .clone()
    }

    /// Drops what was read, so the next ask reads again.
    pub(in crate::inspector) fn forget(&mut self) {
        self.known = None;
    }
}

/// The voice row: what is chosen, what there is to choose, and what is wrong
/// with the choice when something is.
pub(super) fn row(ui: &mut Ui, chosen: Option<&str>, known: &[Voice]) -> Option<String> {
    ui.label("Voice");

    if known.is_empty() {
        ui.label(RichText::new(showing(chosen, known)).strong());
        ui.label(RichText::new("nothing to pick from yet").weak().small());
        ui.end_row();
        return None;
    }

    let mut picked = None;
    ComboBox::from_id_salt("brief-voice")
        .selected_text(showing(chosen, known))
        .show_ui(ui, |ui| {
            for voice in known {
                let this = Some(voice.id.as_str()) == chosen;
                if ui
                    .selectable_label(this, &voice.name)
                    .on_hover_text(describe(voice))
                    .clicked()
                {
                    picked = Some(voice.id.clone());
                }
            }
        });
    ui.label(RichText::new(note(chosen, known)).weak().small());
    ui.end_row();
    picked.filter(|id| Some(id.as_str()) != chosen)
}

/// The sentence for a project with no cached list, and whether the button under
/// it was pressed.
///
/// **The state most people meet first**, so it says the whole thing: what is
/// missing, which command fills it, and why there is no voice already chosen.
/// The last part is the one that stops this reading as a bug — an editor that
/// ships no default voice looks broken until you know that every one of the
/// vendor's expires on a date already published.
pub(super) fn nothing_cached(ui: &mut Ui) -> bool {
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "No voices cached for this project. Run `scorsese voices` to fetch the list — \
             scorsese ships no default voice, because every ElevenLabs default one expires \
             on 2026-12-31.",
        )
        .weak()
        .small(),
    );
    ui.small_button("Look again")
        .on_hover_text("Re-reads the project's cache. Touches no network and spends nothing.")
        .clicked()
}

/// What the picker shows for the current choice.
fn showing(chosen: Option<&str>, known: &[Voice]) -> String {
    let Some(id) = chosen else {
        return NONE.to_owned();
    };
    // The raw id when nothing in the list answers to it, rather than a blank or
    // a substitute. That id is what the document says and what a generation
    // would send, and it is the only thing that makes the state debuggable.
    find(id, known).map_or_else(|| id.to_owned(), |voice| voice.name.clone())
}

/// What the third column says about the choice.
fn note(chosen: Option<&str>, known: &[Voice]) -> String {
    let Some(id) = chosen else {
        // The one field on this panel that is not optional at the point of
        // spending, so it is the one that does not say `(optional)`.
        return String::from("required before this line can be spoken");
    };
    match find(id, known) {
        Some(voice) if !voice.traits.is_empty() => voice.traits.join(", "),
        Some(_) => String::new(),
        // Never silently swapped for one that does exist: narration in a voice
        // nobody chose would sound perfectly fine and be perfectly wrong.
        None => String::from("not in the cached list — it may have been withdrawn"),
    }
}

/// The voice with this id, if the list has one.
fn find<'a>(id: &str, known: &'a [Voice]) -> Option<&'a Voice> {
    known.iter().find(|voice| voice.id == id)
}

/// A voice in a sentence, for the hover.
fn describe(voice: &Voice) -> String {
    let mut said = voice.says();
    if let Some(description) = &voice.description {
        said.push('\n');
        said.push_str(description);
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One voice, as a listing fills one in.
    fn known() -> Vec<Voice> {
        vec![Voice {
            id: String::from("v1"),
            name: String::from("Ana"),
            traits: vec![String::from("pt"), String::from("female")],
            description: None,
            preview: None,
        }]
    }

    /// Nothing chosen says so, and says that it has to be — this is the one
    /// field on the panel that is not optional at the point of spending.
    #[test]
    fn no_voice_chosen_reads_as_a_requirement_rather_than_a_default() {
        assert_eq!(showing(None, &known()), NONE);
        assert!(note(None, &known()).contains("required"));
    }

    /// The case with a date on it: every ElevenLabs default voice expires on
    /// 2026-12-31, so an id the list does not know is expected rather than
    /// broken. It is shown **as itself** and never swapped for one that works —
    /// narration in a voice nobody chose would sound perfectly fine.
    #[test]
    fn a_voice_the_list_does_not_know_is_shown_as_itself_and_flagged() {
        assert_eq!(showing(Some("gone"), &known()), "gone");
        assert!(note(Some("gone"), &known()).contains("withdrawn"));
    }

    /// A voice that is there reads by name, with whatever the listing said
    /// about it beside the picker.
    #[test]
    fn a_voice_that_is_there_reads_by_name() {
        assert_eq!(showing(Some("v1"), &known()), "Ana");
        assert_eq!(note(Some("v1"), &known()), "pt, female");
    }
}
