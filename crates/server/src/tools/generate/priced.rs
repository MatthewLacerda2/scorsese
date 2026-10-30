//! What a `generate` quote takes from the user's credits, brief by brief:
//! the items its token keeps, and their sum.

use scorsese_core::{Asset, AssetId, AudioBlock, Project};
use scorsese_providers::quote::Quote;

use crate::credits::{Product, charge_for};
use crate::tools::quotes::{BriefKind, QuoteItem};

/// What `quote` takes from a balance, in micro-dollars: each charged item
/// priced as the [`Product`] its asset in `project` makes, as its reservation
/// will be (`credits::generations::start`).
pub(super) fn credits(quote: &Quote, project: &Project) -> i64 {
    covered(quote, project).iter().map(|item| item.micros).sum()
}

/// Every brief `quote` would pay for, as its token keeps it: each charged
/// item with its price from the user's credits ([`credits`] is their sum)
/// and the words its asset in `project` would send.
pub(super) fn covered(quote: &Quote, project: &Project) -> Vec<QuoteItem> {
    quote
        .items
        .iter()
        .filter_map(|item| {
            let charge = item.charge.as_ref()?;
            let asset = project.asset(&AssetId::new(item.subject.clone()));
            let micros = asset
                .and_then(Product::of)
                .map_or(0, |product| charge_for(product, charge.cents));
            // A track's prompt describes music, as a shot's does a picture:
            // only a line is spoken as written.
            let brief = match asset.and_then(Asset::audio_block) {
                Some(AudioBlock::Speech) => BriefKind::Line,
                _ => BriefKind::Prompt,
            };
            Some(QuoteItem {
                subject: item.subject.clone(),
                says: item.says.clone(),
                micros,
                brief,
                description: asset
                    .and_then(|asset| asset.prompt.clone())
                    .unwrap_or_default(),
            })
        })
        .collect()
}
