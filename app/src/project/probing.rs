//! Finding out how long the footage is, without the window waiting for it.
//!
//! An asset that reached `project.json` by being written there — which is how
//! an assistant adds one — carries no `media` block, and features that need a
//! source's own length silently skip it. The window is one of the three places
//! that closes that gap, and the only one nobody asked: it probes what it
//! finds unprobed the moment a project is opened.
//!
//! **Never on an interaction.** Probing during a drag was the option weighed
//! and rejected: an ffprobe call in the middle of a trim is exactly the
//! stutter this window exists not to have. So it happens once, on open, on
//! another thread, and the window does not wait for it — the pool is filled in
//! a moment later and the picture never pauses.
//!
//! The worker gets a *copy* of the document and hands back only what it
//! learned, asset by asset. That is what keeps it from being a second writer:
//! the window applies the findings to whatever document it holds by then, and
//! only where the metadata is still missing — so an edit made while the probe
//! ran, or a reload from an assistant's write, cannot be clobbered by an
//! answer to a question asked before it.

use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use scorsese_core::{
    AssetId, MediaMetadata, ProbeOutcome, Project, Reprobe, probe_assets, unprobed_assets,
};
use scorsese_render::Ffprobe;

/// What one asset turned out to be.
type Found = (AssetId, MediaMetadata);

/// A probe of the open project's pool, running somewhere else.
pub(crate) struct Probing {
    /// The findings, once the worker has them. A channel rather than a shared
    /// lock because there is exactly one message and the window must never
    /// block to look for it.
    answer: Receiver<Vec<Found>>,
}

impl Probing {
    /// Starts probing the assets in `project` that carry no metadata.
    ///
    /// `None` when there is nothing to probe, which is the ordinary case for a
    /// project built through `scorsese import` — no thread, no ffprobe, no
    /// work at all. Asking that question first is why it is answered without
    /// touching the filesystem.
    pub(crate) fn start(project: &Project, root: &Path) -> Option<Self> {
        if unprobed_assets(project).is_empty() {
            return None;
        }
        let (sender, answer) = channel();
        let copy = project.clone();
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            // A closed receiver means the window moved on — a different
            // project was opened, or it is shutting down. Nothing to report to
            // and nothing to clean up.
            let _ = sender.send(findings(copy, &root));
        });
        Some(Self { answer })
    }

    /// The findings, if the worker has them yet.
    ///
    /// Polled from the repaint loop, so it never blocks. It does not need a
    /// repaint of its own to be noticed: a window with a project open is
    /// already repainting every poll interval to follow the document on disk,
    /// and this rides on that.
    pub(crate) fn finished(&mut self) -> Option<Vec<Found>> {
        match self.answer.try_recv() {
            Ok(found) => Some(found),
            // Disconnected without a message means the worker died, which is
            // the same as having nothing to say: the pool stays as it was and
            // `scorsese probe` is still there to be run.
            Err(TryRecvError::Disconnected) => Some(Vec::new()),
            Err(TryRecvError::Empty) => None,
        }
    }
}

/// Records what a probe found on the document the window holds *now*.
///
/// Only where the metadata is still missing. An asset that gained a `media`
/// block while the probe was running — because an assistant probed it, or
/// re-imported it — has an answer newer than this one, and overwriting it with
/// an older reading of an older file is the one thing a background job must
/// not do. Reports whether anything actually changed, because a save the
/// document does not need is a write the file watcher will notice.
pub(crate) fn record(project: &mut Project, found: &[Found]) -> bool {
    let mut changed = false;
    for (id, media) in found {
        if let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id)
            && asset.media.is_none()
        {
            asset.media = Some(*media);
            changed = true;
        }
    }
    changed
}

/// The whole of the worker: probe a copy of the pool, and report what each
/// asset turned out to be.
///
/// An ffprobe that cannot be found is not an error to raise here. The window
/// was not asked to do this and nothing is waiting on it; the pool simply
/// stays as it was, and `scorsese assets` still says which assets nobody has
/// looked at.
fn findings(mut project: Project, root: &Path) -> Vec<Found> {
    let Ok(probe) = Ffprobe::discover() else {
        return Vec::new();
    };
    probe_assets(&mut project, root, &probe, Reprobe::Skip)
        .into_iter()
        .filter(|row| row.outcome == ProbeOutcome::Recorded)
        .filter_map(|row| {
            let media = project.asset(&row.id).and_then(|asset| asset.media)?;
            Some((row.id, media))
        })
        .collect()
}
