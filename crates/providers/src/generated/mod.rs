//! What every prompted provider does once a generation is on disk: writes it,
//! and records it against its asset — one set of rules for a shot, a still and
//! a line (#1105).
//!
//! There used to be three copies, one per provider, and they drifted: the
//! still's cleared a `media` block measured off a previous file, and the shot's
//! and the line's kept it. Every local caller probes with `Reprobe::Skip`,
//! which skips an asset that already carries `media`, so a re-spoken longer
//! line kept the old line's length and its clips were fitted to it.
//!
//! What is recorded is what was **worked out**, never what was billed (see
//! [`prices`](crate::prices)). A per-generation ledger (#1059) would be written
//! from [`record`] too, since it is the one moment every provider passes
//! through with the file, its bytes and its figure in hand.

mod prompted;

use std::path::{Path, PathBuf};

use scorsese_core::{AssetId, GenerationState, Project, ProjectPath, hash_bytes};

pub use prompted::Plan;
pub(crate) use prompted::{Prompted, adopt, found, ids, pending, plan};

/// Points the asset at what is now on disk, clears any ticket, and says what
/// it was calculated to cost.
///
/// **No `media` is written**, and a block measured off a *different* file is
/// cleared: what a brief asked for is not a measurement (#249), and the
/// commands that call this probe afterwards — a probe that skips any asset
/// already carrying `media`, so a stale block would be kept for ever.
///
/// A figure already recorded against this same file is kept: finding a
/// generation again does not re-price it, and a still collected from a batch
/// at half price stays at half price. `cents` is `None` when the brief cannot
/// be priced, and then a new file carries no figure rather than the old one's.
pub(crate) fn record(
    project: &mut Project,
    id: &AssetId,
    path: &ProjectPath,
    bytes: &[u8],
    cents: Option<u64>,
) {
    let Some(asset) = project.asset_mut(id) else {
        return;
    };
    let same = asset.path.as_ref() == Some(path);
    if !same {
        asset.media = None;
    }
    if !same || asset.estimated_cost_cents.is_none() {
        asset.estimated_cost_cents = cents;
    }
    asset.path = Some(path.clone());
    asset.state = Some(GenerationState::Generated);
    asset.operation = None;
    if !bytes.is_empty() {
        asset.sha256 = Some(hash_bytes(bytes));
    }
}

/// Writes a generation, atomically, creating `generated/` if it is the first.
///
/// Atomic because the cache depends on it: a generation is named for the hash
/// of its brief, so a truncated file is indistinguishable from a finished one
/// and would be served as the answer for ever after — for something somebody
/// paid for. Each provider's error type converts from [`WriteError`].
pub(crate) fn write(path: &Path, bytes: &[u8]) -> Result<(), WriteError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| WriteError {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    scorsese_core::write::atomically(path, bytes).map_err(|source| WriteError {
        path: path.to_path_buf(),
        source,
    })
}

/// A generation that could not be written into `generated/` — each provider
/// reports it as its own `Write` variant.
#[derive(Debug)]
pub(crate) struct WriteError {
    /// Where it was going.
    pub path: PathBuf,
    /// What the operating system said.
    pub source: std::io::Error,
}

/// One `From` per provider error, each onto its `Write { path, source }`.
macro_rules! write_error_into {
    ($($error:ty),+) => {$(
        impl From<WriteError> for $error {
            fn from(WriteError { path, source }: WriteError) -> Self {
                Self::Write { path, source }
            }
        }
    )+};
}

write_error_into!(
    crate::image::ImageError,
    crate::speech::SpeechError,
    crate::video::GenerationError
);
