//! The lifecycle: sketch → generated, in one call — [`speech`](crate::speech)'s,
//! over tracks.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, AudioBlock, Project, ProjectPath};

use crate::credentials::Budget;
use crate::generated::{self, Prompted, place, write};
use crate::prices;
use crate::spending;

use super::{Brief, Incomplete, MusicError, MusicProvider, Outcome, Plan, Stopped};

/// Composes every track that needs it, and reports what happened to each.
///
/// **Idempotent by construction**: a track whose file already exists is
/// recorded and never sent. Each is held to the ceiling including what this
/// run has already spent. Saving the project is the caller's, even when this
/// returns an error — a track composed before the failure was paid for, and
/// the [`Stopped`] lists it.
pub fn generate(
    project: &mut Project,
    root: &Path,
    provider: &dyn MusicProvider,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, Stopped> {
    let ids = generated::ids::<Tracks>(project);
    let mut done = Vec::new();
    let mut spent = 0;
    for (at, id) in ids.iter().enumerate() {
        match one(project, root, provider, budget.spend(spent), id) {
            Ok(outcome) => {
                spent += outcome.spent_cents();
                done.push((id.clone(), outcome));
            }
            Err(error) => {
                return Err(Stopped {
                    unsent: generated::unsent::<Tracks>(project, root, &ids[at..]),
                    run: done,
                    error,
                });
            }
        }
    }
    Ok(done)
}

/// Points every track whose current brief's output is already on disk at
/// that file — the cache half of a run, with nothing spent.
pub fn adopt(project: &mut Project, root: &Path) -> Vec<AssetId> {
    generated::adopt::<Tracks>(project, root)
}

/// What a run would do with `asset` — see [`Plan`].
pub fn plan(root: &Path, asset: &Asset) -> Plan {
    match Brief::of(asset) {
        Err(why) => Plan::Unready(why.to_string()),
        Ok(brief) if brief.realized(root) => Plan::Realized(brief.output()),
        Ok(_) => Plan::Submit,
    }
}

/// Whether a run over this project's tracks would have anything to do. An
/// incomplete track counts: the pass is where *not yet* is said.
pub fn pending(project: &Project, root: &Path) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| Tracks::claims(asset))
        .any(|asset| !matches!(plan(root, asset), Plan::Realized(_)))
}

/// What a track of `seconds` is expected to cost, in US cents — the one
/// calculation the quote and the ceiling share.
pub fn quote(seconds: u32) -> u64 {
    prices::music(seconds).cents
}

/// Tracks, as [`generated`]'s shared steps see them.
struct Tracks;

impl Prompted for Tracks {
    const KIND: AssetKind = AssetKind::GeneratedAudio;
    type Brief = Brief;
    type Unready = Incomplete;
    type Error = MusicError;

    fn claims(asset: &Asset) -> bool {
        asset.audio_block() == Some(AudioBlock::Music)
    }

    fn brief(_project: &Project, _root: &Path, asset: &Asset) -> Result<Brief, Incomplete> {
        Brief::of(asset)
    }

    fn output(brief: &Brief) -> ProjectPath {
        brief.output()
    }

    fn realized(brief: &Brief, root: &Path) -> bool {
        brief.realized(root)
    }

    fn keep(
        project: &mut Project,
        _root: &Path,
        id: &AssetId,
        path: &ProjectPath,
        bytes: &[u8],
        brief: &Brief,
    ) -> Result<(), MusicError> {
        generated::record(project, id, path, bytes, Some(quote(brief.seconds)));
        Ok(())
    }
}

/// Composes one track, or says why it was not composed. `budget` already
/// counts what this run has spent before reaching it.
fn one(
    project: &mut Project,
    root: &Path,
    provider: &dyn MusicProvider,
    budget: Budget,
    id: &AssetId,
) -> Result<Outcome, MusicError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| MusicError::NoSuchAsset { id: id.clone() })?;
    let brief = match Brief::of(asset) {
        Ok(brief) => brief,
        Err(why) => {
            return Ok(Outcome::Incomplete {
                why: why.to_string(),
            });
        }
    };
    if brief.realized(root) {
        let path = generated::found::<Tracks>(project, root, id, &brief)?;
        return Ok(Outcome::Cached { path });
    }
    let output = brief.output();
    // Placed before it is paid for: an output named for an id that climbs
    // is refused while refusing still costs nothing.
    let target = place(root, &output)?;
    let cents = quote(brief.seconds);
    budget.check(cents)?;
    spending::readable(root)?;
    let bytes = match provider.compose(&brief) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(Outcome::Failed {
                message: error.message,
            });
        }
    };
    write(&target, &bytes)?;
    spending::sent(project, root, id, &brief.digest(), cents)?;
    generated::record(project, id, &output, &bytes, Some(cents));
    Ok(Outcome::Generated {
        path: output,
        bytes: bytes.len(),
        estimated_cost_cents: cents,
    })
}
