//! A shot's prompt longer than Veo reads — warned about here, refused where it
//! would be sent.
//!
//! Veo takes at most [`MAX_PROMPT_TOKENS`] of prompt (#1309). Nothing in the
//! document tokenises, so the length is counted in characters at
//! [`CHARACTERS_PER_PROMPT_TOKEN`], a figure chosen to sit above what natural
//! prose averages: a prompt past [`MAX_PROMPT_CHARACTERS`] is past the vendor's
//! limit in any language, and one the live API would take is not stopped.
//!
//! **A warning here, a refusal at generate.** The limit is new to this
//! document, and loading validates, so refusing it on load would stop a
//! project already holding such a sketch from opening at all — the reason the
//! speech models' own limits are held at generate too (#1240). `scorsese
//! check` says it, and `scorsese_providers`' video brief refuses it by name
//! before anything is sent or quoted.

use std::fmt;

use crate::asset::{
    Asset, AssetId, AssetKind, CHARACTERS_PER_PROMPT_TOKEN, MAX_PROMPT_CHARACTERS,
    MAX_PROMPT_TOKENS,
};
use crate::project::Project;

/// A generated video's prompt longer than Veo reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongPrompt {
    /// The shot asking for it.
    pub asset: AssetId,
    /// How many characters the prompt holds.
    pub found: usize,
}

impl LongPrompt {
    /// The finding for `asset`, when its prompt is past the limit.
    pub fn of(asset: &Asset) -> Option<Self> {
        if asset.kind != AssetKind::GeneratedVideo {
            return None;
        }
        let found = asset.prompt.as_deref()?.chars().count();
        (found > MAX_PROMPT_CHARACTERS).then(|| Self {
            asset: asset.id.clone(),
            found,
        })
    }
}

impl fmt::Display for LongPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "asset `{}` has a prompt of {} characters, and Veo reads at most {MAX_PROMPT_TOKENS} \
             tokens, which is never more than {MAX_PROMPT_CHARACTERS} characters at \
             {CHARACTERS_PER_PROMPT_TOKEN} a token; cut it down before it is generated",
            self.asset, self.found
        )
    }
}

impl Project {
    /// Every shot whose prompt [`LongPrompt`] describes.
    pub fn long_prompts(&self) -> Vec<LongPrompt> {
        self.assets.iter().filter_map(LongPrompt::of).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(characters: usize) -> Asset {
        Asset::sketch(
            AssetId::new("shot"),
            AssetKind::GeneratedVideo,
            "a".repeat(characters),
        )
    }

    #[test]
    fn a_prompt_at_the_limit_passes_and_one_past_it_does_not() {
        assert_eq!(LongPrompt::of(&shot(MAX_PROMPT_CHARACTERS)), None);
        let long = LongPrompt::of(&shot(MAX_PROMPT_CHARACTERS + 1)).expect("too long");
        assert_eq!(long.found, MAX_PROMPT_CHARACTERS + 1);
        assert!(long.to_string().contains("1024 tokens"), "{long}");
    }
}
