//! The one bake entry the surfaces share: one named recipe, or every recipe
//! the project has, and one outcome either way.
//!
//! The CLI's `synth bake` and MCP's `synth_bake` both branch on whether an
//! asset was named and both count what was rendered against what was cached;
//! both now ask this, and word the rows their own way.

use std::path::Path;

use scorsese_core::{AssetId, Project};

use super::{Baked, SynthesisError, bake_asset_unless, bake_pending_unless, never};

/// What one bake did: a row per recipe it was asked about, in the project's
/// order — or the one asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Bakes {
    /// Each recipe and what happened to it.
    pub baked: Vec<(AssetId, Baked)>,
}

impl Bakes {
    /// True when the project has no recipe to bake at all.
    pub fn is_empty(&self) -> bool {
        self.baked.is_empty()
    }

    /// How many were rendered this time, rather than found already baked.
    pub fn fresh(&self) -> usize {
        self.baked.iter().filter(|(_, it)| it.is_fresh()).count()
    }

    /// The closing line: rendered against cached, and the price — always
    /// nothing, which is worth saying beside a tool that otherwise costs.
    pub fn tally(&self) -> String {
        let fresh = self.fresh();
        format!(
            "{fresh} rendered, {} cached, $0.00",
            self.baked.len() - fresh
        )
    }
}

/// [`bake_asset`](super::bake_asset) for `only`, or
/// [`bake_pending`](super::bake_pending) when nothing is named. The project
/// is left describing what is on disk; saving it is the caller's.
pub fn bake(
    project: &mut Project,
    project_root: &Path,
    only: Option<&AssetId>,
) -> Result<Bakes, SynthesisError> {
    bake_unless(project, project_root, only, &never)
}

/// [`bake`] that gives up when `stop` says so, with
/// [`SynthesisError::Stopped`].
pub fn bake_unless(
    project: &mut Project,
    project_root: &Path,
    only: Option<&AssetId>,
    stop: &dyn Fn() -> bool,
) -> Result<Bakes, SynthesisError> {
    let baked = match only {
        Some(id) => {
            let one = bake_asset_unless(project, project_root, id, stop)?;
            vec![(id.clone(), one)]
        }
        None => bake_pending_unless(project, project_root, stop)?,
    };
    Ok(Bakes { baked })
}
