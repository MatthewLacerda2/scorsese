//! What the public pricing page says things cost (#1538), computed from the
//! same tables that charge: each provider's estimate
//! (`scorsese_providers::prices`) times its [`Product`]'s multiplier, through
//! [`charge_for`] — so the page cannot disagree with a quote.
//!
//! A message to the assistant is left out: what one costs depends on the
//! work, and no fixed request stands for it honestly.

use scorsese_core::{ClipSeconds, ImageRequest, SpeechModel, VideoRequest};
use scorsese_providers::prices;
use serde::Serialize;

use super::{Product, charge_for};

/// The characters of the line of narration the page prices.
pub const LINE_CHARACTERS: usize = 200;

/// The characters of the prompt the page's still is priced with: a
/// paragraph, which is what the assistant writes for one.
pub const STILL_PROMPT_CHARACTERS: usize = 300;

/// The seconds of music the page prices.
pub const MUSIC_SECONDS: u32 = 40;

/// One example: what it costs a user, one amount or a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Example {
    /// What it is: `image`, `narration`, `music` or `video`.
    pub id: &'static str,
    /// What it costs, or the least it costs, in micro-dollars.
    pub from_micros: i64,
    /// The most it costs, in micro-dollars, when it is a range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_micros: Option<i64>,
}

/// The page's examples, at today's tables: a still at the default size, a
/// [`LINE_CHARACTERS`] line of narration from the fast voice to the
/// expressive one, [`MUSIC_SECONDS`] of music, and 8 seconds of Veo at the
/// default quality. An example the tables have no price for is left out,
/// never shown as zero.
pub fn examples() -> Vec<Example> {
    let one = |id, cents: Option<u64>, product| {
        cents.map(|cents| Example {
            id,
            from_micros: charge_for(product, cents),
            to_micros: None,
        })
    };
    let line = |model| {
        prices::speech(model, LINE_CHARACTERS)
            .ok()
            .map(|estimate| charge_for(Product::SpokenLine, estimate.cents))
    };
    let narration = match (line(SpeechModel::Fast), line(SpeechModel::Expressive)) {
        (Some(fast), Some(expressive)) => Some(Example {
            id: "narration",
            from_micros: fast.min(expressive),
            to_micros: Some(fast.max(expressive)).filter(|most| *most != fast.min(expressive)),
        }),
        _ => None,
    };
    let still = prices::image(&ImageRequest::default(), STILL_PROMPT_CHARACTERS, 0).ok();
    let shot = VideoRequest {
        seconds: ClipSeconds::Eight,
        ..VideoRequest::default()
    };
    [
        one("image", still.map(|e| e.cents), Product::Still),
        narration,
        one(
            "music",
            Some(prices::music(MUSIC_SECONDS).cents),
            Product::MusicTrack,
        ),
        one(
            "video",
            prices::estimate(&shot).ok().map(|e| e.cents),
            Product::VeoShot,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(id: &str) -> Example {
        let all = examples();
        *all.iter()
            .find(|example| example.id == id)
            .unwrap_or_else(|| panic!("{id} is priced: {all:?}"))
    }

    #[test]
    fn every_example_is_what_a_quote_would_charge() {
        let shot = prices::estimate(&VideoRequest {
            seconds: ClipSeconds::Eight,
            ..VideoRequest::default()
        })
        .unwrap();
        assert_eq!(shot.seconds, 8);
        assert_eq!(
            find("video").from_micros,
            charge_for(Product::VeoShot, shot.cents)
        );
        let music = prices::music(40);
        assert_eq!(
            find("music").from_micros,
            charge_for(Product::MusicTrack, music.cents)
        );
        let still = prices::image(&ImageRequest::default(), 300, 0).unwrap();
        assert_eq!(
            find("image").from_micros,
            charge_for(Product::Still, still.cents)
        );
    }

    #[test]
    fn narration_runs_from_the_fast_voice_to_the_expressive_one() {
        let narration = find("narration");
        let fast = prices::speech(SpeechModel::Fast, 200).unwrap();
        let expressive = prices::speech(SpeechModel::Expressive, 200).unwrap();
        assert_eq!(
            narration.from_micros,
            charge_for(Product::SpokenLine, fast.cents)
        );
        assert_eq!(
            narration.to_micros,
            Some(charge_for(Product::SpokenLine, expressive.cents))
        );
    }

    #[test]
    fn the_examples_come_in_the_pages_order() {
        let ids: Vec<_> = examples().iter().map(|example| example.id).collect();
        assert_eq!(ids, ["image", "narration", "music", "video"]);
    }
}
