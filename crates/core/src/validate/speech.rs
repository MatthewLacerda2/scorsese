//! Generated-speech checks: what the vendor will not take, and what it will
//! take and then ignore.
//!
//! The sibling of [`super::video`], with one difference worth stating because
//! it is why this pass is not simply folded in there. Every video check refuses
//! a request the provider would refuse anyway, only sooner and with the asset
//! named. One check here refuses a request the provider would **accept** — a
//! language pinned on the model that drops the field — and that one has no
//! other place it could live. There is no error to catch, no failed run to
//! read: the narration arrives, in the wrong language, having been paid for.

use crate::asset::{Asset, AssetKind, MAX_CHARACTERS};

use super::error::{AssetProblem, SpeechProblem};
use super::field::AssetField;

pub(super) fn check(asset: &Asset, errors: &mut Vec<AssetProblem>) {
    if asset.speech.is_some() && asset.kind != AssetKind::GeneratedAudio {
        errors.push(AssetProblem::StrayField {
            asset: asset.id.clone(),
            field: AssetField::Speech,
            kind: asset.kind,
        });
        return;
    }
    if asset.kind != AssetKind::GeneratedAudio {
        return;
    }

    let mut found = Vec::new();
    let request = asset.speech_request();

    if let Some(conflict) = request.conflict() {
        found.push(SpeechProblem::LanguageIgnored {
            asset: asset.id.clone(),
            model: conflict.model,
            language: conflict.language,
        });
    }
    if request.voice_id.as_ref().is_some_and(|id| id.is_empty()) {
        found.push(SpeechProblem::BlankVoice {
            asset: asset.id.clone(),
        });
    }
    // Counted on the prompt rather than on the block, and so asked of every
    // narration whether it carries a `speech` block or not: an oversized line
    // is unsendable regardless of which voice would have read it.
    //
    // Characters as the vendor bills them, which is `chars()` and not `len()`.
    // Portuguese is one of the two target languages and its accented letters
    // are two bytes each, so counting bytes would refuse a legal script a
    // quarter short of the limit — and would have priced it wrong as well.
    if let Some(prompt) = &asset.prompt {
        let characters = prompt.chars().count();
        if characters > MAX_CHARACTERS {
            found.push(SpeechProblem::TooLong {
                asset: asset.id.clone(),
                found: characters,
                max: MAX_CHARACTERS,
            });
        }
    }

    errors.extend(found.into_iter().map(Into::into));
}
