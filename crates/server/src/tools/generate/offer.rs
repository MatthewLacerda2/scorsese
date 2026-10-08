//! The half-price batch on the hosted server (#947, the web side of #894):
//! what a quote for *now* offers beside itself, and what it says about a still
//! waiting in a batch this server did not order.
//!
//! **The offer is a second quote.** When stills worth
//! [`OFFER_FROM_CENTS`] or more would be drawn now, and stills are all there
//! is to pay for — a batch takes nothing else (`quote::batch`) — the same call
//! also quotes them as a batch and issues that its own token, under
//! `Spend::Batch`. The assistant's confirmation box shows both prices and
//! takes whichever the person picks (`crate::assistant`); either token spends
//! only what it was quoted for, and the other is withdrawn. Never defaulted,
//! and never the assistant's to pick: it cannot confirm either.

use std::path::Path;

use scorsese_core::{AssetId, AssetKind, Project};
use scorsese_providers::image::{self, Plan};
use scorsese_providers::quote::{OFFER_FROM_CENTS, Quote};

use super::credits;
use crate::credits::dollars;

/// The batch quote to offer beside `now`, if any: `alternative` — the same
/// project quoted as a batch, `None` when it could not be — offered when it
/// has something to pay for, and `now`'s stills come to at least
/// [`OFFER_FROM_CENTS`].
pub(super) fn offered(now: &Quote, alternative: Option<Quote>, project: &Project) -> Option<Quote> {
    let alternative = alternative.filter(|batch| !batch.is_free())?;
    (stills(now, project) >= OFFER_FROM_CENTS).then_some(alternative)
}

/// The line the offer reads as: both prices, from the user's credits, and
/// whose choice it is.
pub(super) fn says(now: &Quote, batch: &Quote, project: &Project) -> String {
    format!(
        "Stills: {} now, or {} in a half-price batch ready within 24 hours. Whoever is paying \
         chooses, and the wait is never chosen for them; to be quoted for the batch alone, \
         call generate with batch.",
        dollars(credits(stills(now, project))),
        dollars(credits(batch.cents()))
    )
}

/// What `quote` charges for stills, in US cents.
fn stills(quote: &Quote, project: &Project) -> u64 {
    quote
        .items
        .iter()
        .filter(|item| {
            project
                .asset(&AssetId::new(item.subject.clone()))
                .is_some_and(|asset| asset.kind == AssetKind::GeneratedImage)
        })
        .filter_map(|item| item.charge.as_ref())
        .map(|charge| charge.cents)
        .sum()
}

/// Say plainly what becomes of a still waiting in a batch ordered somewhere
/// else — a project brought in from a local `.scor` folder. Its batch is that
/// key's, which this server cannot ask after, so it would otherwise read as
/// collected free and sit there for ever. A batch this server orders is a job,
/// never an `operation` in the document.
pub(super) fn foreign(project: &Project, root: &Path, quote: &mut Quote) {
    for item in &mut quote.items {
        let Some(asset) = project.asset(&AssetId::new(item.subject.clone())) else {
            continue;
        };
        if asset.kind == AssetKind::GeneratedImage
            && image::plan(project, root, asset) == Plan::InFlight
        {
            item.says = String::from(
                "waiting in a batch ordered outside this server, which cannot collect it — \
                 collect it where it was ordered (scorsese generate --collect) and bring the \
                 project back; nothing more to pay here",
            );
        }
    }
}
