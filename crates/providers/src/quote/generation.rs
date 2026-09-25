//! What realising a project's prompted briefs would spend.
//!
//! One quote across both vendors, because one call realises both and one yes
//! agrees to both. Each asset is priced by the same `plan` the run decides by,
//! so only a brief that would actually be handed over is charged: one whose
//! output already sits in `generated/`, or whose shot is in flight, costs this
//! run nothing, however much it cost the run that paid for it.
//!
//! Shots first, then lines, each in document order — the order every surface
//! has always printed them in, and the order a person reads a cut in.

use std::path::Path;

use scorsese_core::{Asset, AssetKind, Project};

use super::{Charge, Item, Quote, Spend};
use crate::prices::{self, Unpriced, UnpricedSpeech, dollars};
use crate::{speech, video};

/// A brief asks for something there is no price for, so nothing can be quoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Unquotable {
    /// A shot at a tier and size nobody sells.
    #[error(transparent)]
    Video(#[from] Unpriced),
    /// A line in a model with no published rate.
    #[error(transparent)]
    Speech(#[from] UnpricedSpeech),
}

/// What a `generate` over this project would spend right now.
pub fn generation(project: &Project, root: &Path) -> Result<Quote, Unquotable> {
    let mut items = Vec::new();
    for asset in of_kind(project, AssetKind::GeneratedVideo) {
        items.push(shot(project, root, asset)?);
    }
    for asset in of_kind(project, AssetKind::GeneratedAudio) {
        items.push(line(root, asset)?);
    }
    Ok(Quote {
        spend: Spend::Generation,
        items,
    })
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
