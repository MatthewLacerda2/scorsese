//! The lifecycle: sketch → generated, in one call.
//!
//! [`speech`](crate::speech)'s run, for a picture drawn now: an incomplete
//! brief is that still's outcome rather than the end of the run, and what
//! stops a run is what makes every remaining still impossible. A still waiting
//! in a batch is [`collect`](super::collect)'s, which every run calls first.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath};

use crate::credentials::Budget;
use crate::generated::{self, Plan, Prompted, record, write_in};
use crate::prices::{self, ImageEstimate, UnpricedImage};
use crate::spending;

use super::stopped::stopped;
use super::{Brief, ImageError, ImageProvider, Incomplete, Outcome, Stopped, collect};

/// Draws every still that needs it, and reports what happened to each.
///
/// **Idempotent by construction, and that is the money guarantee**: a still
/// whose file already exists is recorded and never sent. The project is left
/// describing what is on disk; saving it is the caller's, **even when this
/// returns an error** — a still drawn before the failure has been paid for,
/// and the [`Stopped`] it answers with lists it beside the reason (#1153).
///
/// Whatever is waiting in a batch is collected first, and a still the
/// collection touched is not drawn again on the same run, even one whose batch
/// failed: the quote this run was agreed on priced it at nothing.
///
/// **A still that was not ready when the run began is not drawn on it**, even
/// once its reference lands earlier in the same run: the quote said *not yet*,
/// and a confirmed run spends on exactly what was quoted (#1058). It is drawn
/// on the next call, after a quote that prices it.
pub fn generate(
    project: &mut Project,
    root: &Path,
    provider: &dyn ImageProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, Stopped> {
    let unready = unready(project, root);
    let mut done = match collect(project, root, provider) {
        Ok(collected) => collected,
        Err(error) => return Err(stopped(project, root, Vec::new(), &unready, error)),
    };
    let mut spent = 0;
    for id in generated::ids::<Stills>(project) {
        if done.iter().any(|(handled, _)| handled == &id) {
            continue;
        }
        if let Some((_, why)) = unready.iter().find(|(waiting, _)| waiting == &id) {
            done.push((id, Outcome::Incomplete { why: why.clone() }));
            continue;
        }
        match one(project, root, provider, budget.spend(spent), &id) {
            Ok(outcome) => {
                spent += outcome.spent_cents();
                done.push((id, outcome));
            }
            Err(error) => return Err(stopped(project, root, done, &unready, error)),
        }
    }
    Ok(done)
}

/// Points every still whose current brief's output is already on disk at that
/// file — the cache half of a run, with no provider and nothing spent. Which
/// stills it found. The sibling of [`video::adopt`](crate::video::adopt).
pub fn adopt(project: &mut Project, root: &Path) -> Vec<AssetId> {
    generated::adopt::<Stills>(project, root)
}

/// What a run would do with `asset` — see [`Plan`].
pub fn plan(project: &Project, root: &Path, asset: &Asset) -> Plan {
    generated::plan::<Stills>(project, root, asset)
}

/// Whether a run over this project's stills would have anything to do — an
/// incomplete one counts, because the pass is where *not yet* is voiced, and
/// so does any ticket, which the run collects or clears.
pub fn pending(project: &Project, root: &Path) -> bool {
    generated::pending::<Stills>(project, root)
}

/// Every still that cannot be drawn yet, and why — read **before** a run
/// touches anything, because that is what the quote saw.
pub(super) fn unready(project: &Project, root: &Path) -> Vec<(AssetId, String)> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedImage)
        .filter_map(|asset| match plan(project, root, asset) {
            Plan::Unready(why) => Some((asset.id.clone(), why)),
            _ => None,
        })
        .collect()
}

/// What drawing `brief` is expected to cost — the one calculation the quote,
/// the ceiling and the record all use.
pub(crate) fn estimate(brief: &Brief) -> Result<ImageEstimate, UnpricedImage> {
    prices::image(
        &brief.request,
        brief.characters(),
        brief.reference_images.len(),
    )
}

/// Stills, as [`generated`]'s shared steps see them.
pub(super) struct Stills;

impl Prompted for Stills {
    const KIND: AssetKind = AssetKind::GeneratedImage;
    type Brief = Brief;
    type Unready = Incomplete;
    type Error = ImageError;

    fn brief(project: &Project, root: &Path, asset: &Asset) -> Result<Brief, Incomplete> {
        Brief::of(project, root, asset)
    }

    fn output(brief: &Brief) -> ProjectPath {
        brief.output()
    }

    fn realized(brief: &Brief, root: &Path) -> bool {
        brief.realized(root)
    }

    /// [`record`], at the rate a still drawn now costs. One that cannot be
    /// priced is not recorded.
    fn keep(
        project: &mut Project,
        _root: &Path,
        id: &AssetId,
        path: &ProjectPath,
        bytes: &[u8],
        brief: &Brief,
    ) -> Result<(), ImageError> {
        record(project, id, path, bytes, Some(estimate(brief)?.cents));
        Ok(())
    }
}

/// Draws one still, or says why it was not drawn.
fn one(
    project: &mut Project,
    root: &Path,
    provider: &dyn ImageProvider,
    budget: Budget,
    id: &AssetId,
) -> Result<Outcome, ImageError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| ImageError::NoSuchAsset { id: id.clone() })?;
    let brief = match Brief::of(project, root, asset) {
        Ok(brief) => brief,
        Err(why) => {
            return Ok(Outcome::Incomplete {
                why: why.to_string(),
            });
        }
    };

    // The cache first: a file already there is the answer to "has this been
    // paid for", whatever the document claims.
    if brief.realized(root) {
        let path = generated::found::<Stills>(project, root, id, &brief)?;
        return Ok(Outcome::Cached { path });
    }
    let output = brief.output();

    let cents = estimate(&brief)?.cents;
    budget.check(cents)?;
    spending::readable(root)?;

    let bytes = match provider.draw(&brief) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(Outcome::Failed {
                message: error.message,
            });
        }
    };
    write_in(root, &output, &bytes)?;
    spending::sent(project, root, id, &brief.digest(), cents)?;
    record(project, id, &output, &bytes, Some(cents));
    Ok(Outcome::Generated {
        path: output,
        bytes: bytes.len(),
        estimated_cost_cents: cents,
    })
}
