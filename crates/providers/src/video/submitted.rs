//! Which brief a ticket was bought for — kept beside the ticket, under
//! `generated/tickets/` (#1106).
//!
//! **The brief in the document is the current one, not the one that was
//! sent.** A shot edited while it is in flight — in the app's inspector, by
//! `project_write`, by hand — used to have the old prompt's video filed under
//! the new prompt's name when it arrived: the cache then answered the new
//! prompt with the old video for ever, and the new prompt was never sent.
//! Remembering what was sent is what lets the arriving video be filed where it
//! belongs and the asset released for a new quote.
//!
//! A sidecar rather than a field on the asset, because a field is a
//! `project.json` format change. It names the ticket it belongs to, so one left
//! behind by an older ticket is never believed; a ticket with no sidecar it
//! answers to (submitted before this existed, or by the hosted server) is
//! filed under the current brief, as it always was.

use std::path::{Path, PathBuf};

use scorsese_core::{AssetId, GENERATED_DIR, ProjectPath};

use crate::generated::{WriteError, write};

/// The folder under `generated/` the sidecars live in.
const TICKETS_DIR: &str = "tickets";

/// Remembers that `operation` was bought for the brief whose digest is
/// `digest`.
pub(crate) fn remember(
    root: &Path,
    id: &AssetId,
    operation: &str,
    digest: &str,
) -> Result<(), WriteError> {
    write(
        &sidecar(root, id),
        format!("{operation}\n{digest}\n").as_bytes(),
    )
}

/// Where the video `operation` was bought for lands — `None` when nothing
/// answering to that ticket was remembered.
pub(crate) fn output(root: &Path, id: &AssetId, operation: &str) -> Option<ProjectPath> {
    let text = std::fs::read_to_string(sidecar(root, id)).ok()?;
    let mut lines = text.lines();
    if lines.next()? != operation {
        return None;
    }
    // Only a digest is believed, never a path: a hand-edited sidecar must not
    // be able to point a write anywhere but at a shot's own file.
    let digest = lines.next()?;
    let plain = !digest.is_empty() && digest.bytes().all(|byte| byte.is_ascii_hexdigit());
    plain.then(|| ProjectPath::new(format!("{GENERATED_DIR}/{id}-{digest}.mp4")))
}

/// Forgets the asset's ticket. Nothing to forget is not an error.
pub(crate) fn forget(root: &Path, id: &AssetId) {
    let _ = std::fs::remove_file(sidecar(root, id));
}

/// The sidecar's file.
fn sidecar(root: &Path, id: &AssetId) -> PathBuf {
    root.join(GENERATED_DIR).join(TICKETS_DIR).join(id.as_str())
}
