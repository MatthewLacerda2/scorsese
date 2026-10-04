//! How dear each model is beside the others, as the picker's bar shows it
//! (#706): a share of the dearest, never a price.
//!
//! A turn is charged for input and output separately, and the two do not
//! scale together across models (Opus is about 13× Flash Lite on input but
//! 8× on output), so neither alone orders them honestly. They are blended at
//! a typical turn's mix: **nine input tokens to one output token**, because
//! every tool reply goes back in as input. That mix is fixed until measured
//! usage says otherwise (#707).
//!
//! Computed here rather than in the browser so the rate table stays the one
//! source of every number: the front-end gets a figure to draw, and a price
//! change moves the bars without a front-end change.

use scorsese_providers::chat::Model;
use scorsese_providers::prices::chat::Rate;

/// Input tokens per output token in the blend.
const INPUT_PER_OUTPUT: u64 = 9;

/// What `rate` charges for ten blended tokens: nine in, one out. Only ever
/// compared with another blend, so its unit does not matter.
fn blended(rate: Rate) -> u64 {
    rate.input
        .saturating_mul(INPUT_PER_OUTPUT)
        .saturating_add(rate.output)
}

/// Each of `models`' blended cost as a percentage of the dearest one's,
/// rounded to the nearest and never below 1, in `models`' order. `None` for a
/// model `rate` has no price for, which is then left out of the comparison.
pub(super) fn relative(models: &[Model], rate: impl Fn(Model) -> Option<Rate>) -> Vec<Option<u8>> {
    let blends: Vec<Option<u64>> = models.iter().map(|&m| rate(m).map(blended)).collect();
    let dearest = blends.iter().flatten().copied().max().unwrap_or(0);
    blends
        .into_iter()
        .map(|blend| {
            let blend = blend?;
            if dearest == 0 {
                return Some(1);
            }
            let percent = (u128::from(blend) * 100 + u128::from(dearest) / 2) / u128::from(dearest);
            Some(u8::try_from(percent.clamp(1, 100)).unwrap_or(100))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use scorsese_providers::prices::chat::rate;

    use super::*;

    #[test]
    fn the_table_orders_the_models_at_nine_in_one_out() {
        let models = [
            Model::ClaudeOpus55,
            Model::ClaudeSonnet55,
            Model::GeminiFlash38,
            Model::GeminiFlashLite35,
        ];
        // $5.60, $2.80, $1.05 and $0.52 per blended million (#706).
        assert_eq!(
            relative(&models, rate),
            [Some(100), Some(50), Some(19), Some(9)]
        );
    }

    #[test]
    fn a_changed_rate_moves_a_bar() {
        let models = [Model::ClaudeOpus55, Model::GeminiFlash38];
        let doubled = |model| {
            let mut found = rate(model)?;
            if model == Model::GeminiFlash38 {
                found.input *= 2;
                found.output *= 2;
            }
            Some(found)
        };
        assert_eq!(relative(&models, doubled), [Some(100), Some(38)]);
    }

    #[test]
    fn the_dearest_is_whichever_costs_most_and_an_unpriced_model_has_no_bar() {
        let models = [Model::GeminiFlash38, Model::GeminiFlashLite35];
        let only_flash = |model| {
            (model == Model::GeminiFlash38)
                .then(|| rate(model))
                .flatten()
        };
        assert_eq!(relative(&models, only_flash), [Some(100), None]);
        assert_eq!(relative(&models, rate), [Some(100), Some(50)]);
    }
}
