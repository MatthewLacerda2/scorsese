//! Whether a turn is offered Veo (#1530): only with paid credit, which a shot
//! is paid from, never the free allowance. Without it the turn's `generate`
//! says nothing of video (`Toolbox::listing_for`) and the conversation is
//! told why — once per change, since its history is replayed every turn.

use scorsese_providers::chat::Message;

/// What the conversation is told when Veo goes.
pub(super) const NO_VEO: &str = "Veo is not available because the user is on free usage: \
     there are no Veo shots to generate. Hold, push in or pan on a generated_image, or use \
     stock footage, instead.";

/// What it is told when Veo comes back.
pub(super) const VEO_BACK: &str = "Veo is available again: the user now has paid credit, so \
     Veo shots can be generated.";

/// Whether the conversation was last told Veo is available: until told
/// otherwise, it is.
pub(super) fn told(said: &[Message]) -> bool {
    said.iter()
        .rev()
        .find_map(|message| match message {
            Message::System { text } if text.contains(NO_VEO) => Some(false),
            Message::System { text } if text.contains(VEO_BACK) => Some(true),
            _ => None,
        })
        .unwrap_or(true)
}

/// What a turn offered Veo or not (`veo`) must say first, after `said`.
pub(super) fn note(said: &[Message], veo: bool) -> Option<&'static str> {
    (told(said) != veo).then_some(if veo { VEO_BACK } else { NO_VEO })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system(text: &str) -> Message {
        Message::System { text: text.into() }
    }

    #[test]
    fn the_note_is_said_once_per_change() {
        assert_eq!(note(&[], true), None);
        assert_eq!(note(&[], false), Some(NO_VEO));
        let gone = [system(&format!("about the project\n\n{NO_VEO}"))];
        assert_eq!(note(&gone, false), None);
        assert_eq!(note(&gone, true), Some(VEO_BACK));
        // Any other note since says nothing about Veo.
        let since = [gone[0].clone(), system("the person edited the script")];
        assert_eq!(note(&since, false), None);
        let back = [gone[0].clone(), system(VEO_BACK)];
        assert_eq!(note(&back, true), None);
        assert_eq!(note(&back, false), Some(NO_VEO));
    }
}
