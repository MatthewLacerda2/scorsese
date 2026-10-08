//! What realising a project's prompted briefs would spend.
//!
//! One quote across every vendor, because one call realises them all and one
//! yes agrees to all of it. Each asset is priced by the same `plan` the run decides by,
//! so only a brief that would actually be handed over is charged: one whose
//! output already sits in `generated/`, or whose shot is in flight, costs this
//! run nothing, however much it cost the run that paid for it.
//!
//! Shots first, then stills, then lines, each in document order — the order every surface
//! has always printed them in, and the order a person reads a cut in.
//!
//! **A batch is quoted on its own** ([`batch`], #894): stills only, at half
//! the rate, under its own [`Spend`] so a token for one order is never spent
//! on the other. A request to batch anything else is refused, naming it.
//! [`offer`] is what the quote for *now* shows beside itself, so the person
//! chooses with the saving in front of them.

use std::path::Path;

use scorsese_core::{Asset, AssetKind, Project};

use super::{Charge, Item, Quote, Spend};
use crate::image::Order;
use crate::prices::{self, Unpriced, UnpricedImage, UnpricedSpeech, dollars};
use crate::{image, speech, video};

/// Below this many cents of stills drawn now, the quote does not offer a
/// batch: the saving is under half a dollar, not worth a day's wait or the
/// question (the maintainer's *about a dollar*, #894).
pub const OFFER_FROM_CENTS: u64 = 100;

/// A brief asks for something there is no price for, so nothing can be quoted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unquotable {
    /// A shot at a tier and size nobody sells.
    #[error(transparent)]
    Video(#[from] Unpriced),
    /// A line in a model with no published rate.
    #[error(transparent)]
    Speech(#[from] UnpricedSpeech),
    /// A still at a size its model does not draw.
    #[error(transparent)]
    Image(#[from] UnpricedImage),
    /// A batch asked for with a shot or a line in it that would be sent.
    #[error(
        "only stills can wait in a half-price batch, and {} would be sent too — generate \
         without batch, or once only stills are left to draw",
        .0.join(", ")
    )]
    Unbatchable(
        /// The shots and lines, by id.
        Vec<String>,
    ),
}

/// What a `generate` over this project would spend right now.
pub fn generation(project: &Project, root: &Path) -> Result<Quote, Unquotable> {
    let mut items = Vec::new();
    for asset in of_kind(project, AssetKind::GeneratedVideo) {
        items.push(shot(project, root, asset)?);
    }
    for asset in of_kind(project, AssetKind::GeneratedImage) {
        items.push(still(project, root, asset, Order::Now)?);
    }
    for asset in of_kind(project, AssetKind::GeneratedAudio) {
        items.push(line(root, asset)?);
    }
    Ok(Quote {
        spend: Spend::Generation,
        items,
    })
}

/// What a `generate` ordering this project's stills as a half-price batch
/// would spend — refused when a shot or a line would be sent with them.
pub fn batch(project: &Project, root: &Path) -> Result<Quote, Unquotable> {
    let now = generation(project, root)?;
    let others: Vec<String> = now
        .items
        .iter()
        .filter(|item| item.charge.is_some())
        .filter(|item| {
            project.assets.iter().any(|asset| {
                asset.id.as_str() == item.subject && asset.kind != AssetKind::GeneratedImage
            })
        })
        .map(|item| item.subject.clone())
        .collect();
    if !others.is_empty() {
        return Err(Unquotable::Unbatchable(others));
    }
    let mut items = Vec::new();
    for asset in of_kind(project, AssetKind::GeneratedImage) {
        items.push(still(project, root, asset, Order::Batch)?);
    }
    Ok(Quote {
        spend: Spend::Batch,
        items,
    })
}

/// The stills of a quote for now, priced both ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    /// What they cost drawn now, in US cents.
    pub now_cents: u64,
    /// What they cost in a batch, in US cents.
    pub batch_cents: u64,
}

impl Offer {
    /// The line a quote shows: both prices side by side, and that the choice
    /// is the person's — it trades their time.
    pub fn says(&self) -> String {
        format!(
            "Stills: {} now · {} in a batch, ready within 24 hours (half price). Ask whoever \
             is paying which they'd rather — never choose the wait for them.",
            dollars(self.now_cents),
            dollars(self.batch_cents)
        )
    }
}

/// Whether to offer a batch beside this project's quote for now: when it has
/// stills to draw worth at least [`OFFER_FROM_CENTS`]. A batch needs nothing
/// else in the call, but the offer is made either way, since the stills can be
/// ordered on their own.
pub fn offer(project: &Project, root: &Path) -> Result<Option<Offer>, Unquotable> {
    let mut now_cents = 0;
    let mut batch_cents = 0;
    for asset in of_kind(project, AssetKind::GeneratedImage) {
        let charged = |order| -> Result<u64, Unquotable> {
            Ok(still(project, root, asset, order)?
                .charge
                .map_or(0, |charge| charge.cents))
        };
        now_cents += charged(Order::Now)?;
        batch_cents += charged(Order::Batch)?;
    }
    Ok((now_cents >= OFFER_FROM_CENTS).then_some(Offer {
        now_cents,
        batch_cents,
    }))
}

/// Every asset of one kind, in document order.
fn of_kind(project: &Project, kind: AssetKind) -> impl Iterator<Item = &Asset> {
    project
        .assets
        .iter()
        .filter(move |asset| asset.kind == kind)
}

/// One shot, as the run would treat it.
fn shot(project: &Project, root: &Path, asset: &Asset) -> Result<Item, Unquotable> {
    let free = |says: String| Item {
        subject: asset.id.to_string(),
        says,
        charge: None,
    };
    Ok(match video::plan(project, root, asset) {
        video::Plan::Realized(path) => free(format!("already generated — {path} — nothing to pay")),
        video::Plan::InFlight => free(String::from(
            "still generating — already paid for, collected free",
        )),
        video::Plan::Unready(why) => free(format!("would not be sent — {why}")),
        video::Plan::Submit => {
            // `plan` gathered this brief a moment ago to decide on Submit, so
            // gathering it again answers the same way.
            let brief = match video::Brief::of(project, root, asset) {
                Ok(brief) => brief,
                Err(why) => return Ok(free(format!("would not be sent — {why}"))),
            };
            let priced = prices::estimate(&brief.request)?;
            Item {
                subject: asset.id.to_string(),
                says: format!(
                    "{} — {}s of {} at {}",
                    dollars(priced.cents),
                    priced.seconds,
                    brief.request.model.as_str(),
                    brief.request.resolution.as_str()
                ),
                charge: Some(Charge {
                    brief: brief.digest(),
                    cents: priced.cents,
                }),
            }
        }
    })
}

/// One still, as the run would treat it. The price is the picture's, fixed by
/// its size, plus the input it is sent with — see [`prices::image`] for what
/// that counts and what it does not — halved in a batch.
fn still(project: &Project, root: &Path, asset: &Asset, order: Order) -> Result<Item, Unquotable> {
    let free = |says: String| Item {
        subject: asset.id.to_string(),
        says,
        charge: None,
    };
    Ok(match image::plan(project, root, asset) {
        image::Plan::Realized(path) => free(format!("already drawn — {path} — nothing to pay")),
        image::Plan::Unready(why) => free(format!("not yet — {why}")),
        image::Plan::InFlight => free(String::from(
            "waiting in its batch — already paid for, collected free",
        )),
        image::Plan::Submit => {
            let brief = match image::Brief::of(project, root, asset) {
                Ok(brief) => brief,
                Err(why) => return Ok(free(format!("not yet — {why}"))),
            };
            let price = match order {
                Order::Now => prices::image,
                Order::Batch => prices::image_in_batch,
            };
            let priced = price(
                &brief.request,
                brief.characters(),
                brief.reference_images.len(),
            )?;
            let when = match order {
                Order::Now => "",
                Order::Batch => ", in a batch — ready within 24 hours",
            };
            let references = match brief.reference_images.len() {
                0 => String::new(),
                1 => String::from(", from 1 reference"),
                n => format!(", from {n} references"),
            };
            Item {
                subject: asset.id.to_string(),
                says: format!(
                    "{} — a {} {} still in {}{references}{when}",
                    dollars(priced.cents),
                    brief.request.size().as_str(),
                    brief.request.aspect.as_str(),
                    brief.request.model.as_str()
                ),
                charge: Some(Charge {
                    brief: brief.digest(),
                    cents: priced.cents,
                }),
            }
        }
    })
}

/// One line of narration, as the run would treat it.
///
/// Exact before anything is sent, because the vendor bills by character; still
/// an estimate, since the rate is a page somebody copied and library-voice
/// multipliers are deliberately ignored.
fn line(root: &Path, asset: &Asset) -> Result<Item, Unquotable> {
    let free = |says: String| Item {
        subject: asset.id.to_string(),
        says,
        charge: None,
    };
    Ok(match speech::plan(root, asset) {
        speech::Plan::Realized(path) => free(format!("already spoken — {path} — nothing to pay")),
        speech::Plan::Unready(why) => free(format!("not yet — {why}")),
        speech::Plan::Submit => {
            let brief = match speech::Brief::of(asset) {
                Ok(brief) => brief,
                Err(why) => return Ok(free(format!("not yet — {why}"))),
            };
            let priced = prices::speech(brief.request.model, brief.characters())?;
            Item {
                subject: asset.id.to_string(),
                says: format!(
                    "{} — {} characters in {}",
                    dollars(priced.cents),
                    priced.characters,
                    brief.request.model.as_str()
                ),
                charge: Some(Charge {
                    brief: brief.digest(),
                    cents: priced.cents,
                }),
            }
        }
    })
}
