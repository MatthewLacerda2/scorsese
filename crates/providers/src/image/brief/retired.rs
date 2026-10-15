//! Stills drawn by a model scorsese no longer offers (#1308).
//!
//! Until v50 a still named its model — `nano_banana_2.1`, `flash`, `lite` or
//! `pro` — and the fingerprint hashed that name. Now every still is 2.1's, and
//! the v49 → v50 migration drops the field, so a still drawn by one of the
//! other three hashes to a new digest and its picture is no longer at its
//! brief's [`output`](super::Brief::output). Left there, the next run would
//! draw it again and bill for it; the maintainer was told a still already
//! drawn stays drawn.
//!
//! **The document's `generated` is not enough to go on.** A run trusts the
//! file a brief names, never the state, because the state cannot see a
//! reference that was redrawn under the same id — a still drawn from it must
//! be drawn again, and only the fingerprint notices. So a retired still is
//! recognised the same way: its file is the one its brief, written as that
//! model wrote it, would have been named — so the prompt, the size, the aspect,
//! the thinking and every reference's bytes are as they were when it was drawn.
//!
//! What the migration had to change is covered too: a 0.5K Flash still is
//! `1K` now, and Lite's objects past the tenth moved to the characters, in the
//! same order — the order is all the fingerprint sees. A Pro still that named
//! a style or a fifth character lost that picture, so its brief is not the one
//! it was drawn from, and it is drawn again by 2.1 at the next run, quoted
//! first like any other.

use std::path::Path;

use scorsese_core::{Asset, GenerationState, ImageResolution, ImageThinking, ProjectPath};

use super::Brief;
use crate::fingerprint;

/// Each retired model as its fingerprint spelled it, with the size it drew and
/// the level it thought at when the request named none (Pro had no level).
const RETIRED: [(&str, ImageResolution, Option<ImageThinking>); 3] = [
    ("flash", ImageResolution::K2, Some(ImageThinking::Minimal)),
    ("lite", ImageResolution::K1, Some(ImageThinking::Minimal)),
    ("pro", ImageResolution::K2, None),
];

/// The file `asset` already plays, when the document says it is drawn and
/// that file is the one a retired model drew for exactly this brief.
pub(super) fn drawn(brief: &Brief, asset: &Asset, root: &Path) -> Option<ProjectPath> {
    if asset.state != Some(GenerationState::Generated) {
        return None;
    }
    let path = asset.path.as_ref()?;
    let found = RETIRED.iter().any(|&(model, size, thinking)| {
        sizes(brief, size).into_iter().any(|size| {
            let digest = brief
                .written_as(model, size, level(brief, thinking))
                .digest();
            *path == fingerprint::output(&brief.id, &digest, "jpg")
        })
    });
    (found && path.resolve(root).is_file()).then(|| path.clone())
}

/// The sizes the still may have been drawn at: as written, or the model's
/// default when absent — and `0.5K` too when it now reads `1K`, which is
/// what the migration made of it.
fn sizes(brief: &Brief, default: ImageResolution) -> Vec<&'static str> {
    match brief.request.resolution {
        None => vec![default.as_str()],
        Some(ImageResolution::K1) => vec!["1K", "0.5K"],
        Some(size) => vec![size.as_str()],
    }
}

/// The thinking line the model wrote: the level asked for, only when it was
/// not that model's default.
fn level(brief: &Brief, default: Option<ImageThinking>) -> Option<&'static str> {
    let level = brief.request.thinking.or(default);
    level
        .filter(|_| level != default)
        .map(ImageThinking::as_str)
}
