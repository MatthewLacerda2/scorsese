//! What every pass of one `generate` did ([`super::passes`]): the outcomes
//! of each, the one total that spans them, and what landed to be measured.

use std::path::Path;

use scorsese_core::placing::{self, Shortened};
use scorsese_core::{AssetId, ProbeMedia, Project, Reprobe, probe_assets};

use crate::video::{self, Run};
use crate::{image, music, sound, speech};

/// What every pass did, a stopped one's included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    /// The shots, and what they committed.
    pub shots: Run,
    /// The stills.
    pub stills: Vec<(AssetId, image::Outcome)>,
    /// The narration.
    pub lines: Vec<(AssetId, speech::Outcome)>,
    /// The music (#1264).
    pub tracks: Vec<(AssetId, music::Outcome)>,
    /// The sound effects (#1265).
    pub sounds: Vec<(AssetId, sound::Outcome)>,
}

impl Default for Done {
    /// A run that has done nothing yet.
    fn default() -> Self {
        Self {
            shots: Run {
                outcomes: Vec::new(),
                spent_cents: 0,
            },
            stills: Vec::new(),
            lines: Vec::new(),
            tracks: Vec::new(),
            sounds: Vec::new(),
        }
    }
}

impl Done {
    /// Whether no pass did anything at all.
    pub fn is_empty(&self) -> bool {
        self.shots.outcomes.is_empty()
            && self.stills.is_empty()
            && self.lines.is_empty()
            && self.tracks.is_empty()
            && self.sounds.is_empty()
    }

    /// What the stills committed.
    pub(super) fn stills_spent(&self) -> u64 {
        self.stills.iter().map(|(_, o)| o.spent_cents()).sum()
    }

    /// What the whole run is estimated to have spent, in US cents — the one
    /// total that spans every provider. Our arithmetic, never a bill.
    pub fn spent_cents(&self) -> u64 {
        self.shots.spent_cents + self.stills_spent() + self.voiced_spent()
    }

    /// What the lines, the tracks and the sounds committed — the ElevenLabs
    /// passes.
    pub(super) fn voiced_spent(&self) -> u64 {
        let lines: u64 = self.lines.iter().map(|(_, o)| o.spent_cents()).sum();
        let tracks: u64 = self.tracks.iter().map(|(_, o)| o.spent_cents()).sum();
        let sounds: u64 = self.sounds.iter().map(|(_, o)| o.spent_cents()).sum();
        lines + tracks + sounds
    }

    /// The shots, lines, tracks and sounds that arrived on disk with a length nothing has
    /// measured yet — the ones whose clips may now outlast them.
    ///
    /// A cache hit is not one of them: its file was already there, and
    /// whatever ran when it first landed has had every chance to look at it.
    pub fn landed(&self) -> Vec<AssetId> {
        let shot = self
            .shots
            .outcomes
            .iter()
            .filter(|(_, outcome)| matches!(outcome, video::Outcome::Generated { .. }));
        let line = self
            .lines
            .iter()
            .filter(|(_, outcome)| matches!(outcome, speech::Outcome::Generated { .. }));
        let track = self
            .tracks
            .iter()
            .filter(|(_, outcome)| matches!(outcome, music::Outcome::Generated { .. }));
        let effect = self
            .sounds
            .iter()
            .filter(|(_, outcome)| matches!(outcome, sound::Outcome::Generated { .. }));
        shot.map(|(id, _)| id)
            .chain(line.map(|(id, _)| id))
            .chain(track.map(|(id, _)| id))
            .chain(effect.map(|(id, _)| id))
            .cloned()
            .collect()
    }

    /// Whether anything landed that has to be measured before the project
    /// is saved for good — a shot, a line or a still.
    pub fn unmeasured(&self) -> bool {
        !self.landed().is_empty() || self.stills.iter().any(|(_, o)| o.landed())
    }

    /// Probes everything unmeasured and shortens the clips of what
    /// [`landed`](Self::landed) that now outlast it, answering with each.
    ///
    /// **A generation has no measured shape until something looks**, and this
    /// crate has no decoder and must not grow one, so the prober is handed in.
    /// Measuring is also what can make the document refuse to load — a clip
    /// laid out over a sketch may outlast what came back — so the clips are
    /// fitted here, before the caller saves ([`placing::fit_to_sources`],
    /// #825, #1114).
    pub fn measure(
        &self,
        project: &mut Project,
        root: &Path,
        probe: &dyn ProbeMedia,
    ) -> Vec<Shortened> {
        probe_assets(project, root, probe, Reprobe::Skip);
        placing::fit_to_sources(project, &self.landed())
    }
}
