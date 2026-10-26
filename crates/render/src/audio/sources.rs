//! The decoders a mix keeps open from one stretch of timeline to the next.
//!
//! A clip is heard across every stretch from its start to its end, and the
//! timeline is cut wherever *any* clip starts or stops: a music bed under three
//! narration lines plays through seven stretches. Each one decoding the bed
//! again would seek into it at every seam, and an input seek into a compressed
//! sound does not land sample-exact — an mp3 opens on ~31 ms of near-silence,
//! an `.m4a` drops a frame (#1607). So a clip's sound is decoded **once**, from
//! where it enters the first of its stretches for as long as it stays, and
//! each stretch reads its share off the same stream. That one decode opens a
//! little early and cuts to the point exactly, so a clip entering partway into
//! its file — a bed split in two, or trimmed at its head — has no hole either.

use std::collections::BTreeMap;

use scorsese_core::TrackId;

use crate::error::RenderError;
use crate::pipe::{AudioDecoder, AudioSource};
use crate::plan::{Plan, Shot};
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// The open decoders, one per clip heard in the stretch being mixed.
pub(super) struct Sources<'r, 'a> {
    tools: &'r Tools,
    settings: &'r RenderSettings,
    plan: &'r Plan<'a>,
    open: BTreeMap<Key<'a>, Open>,
}

/// A clip by its track, then its start, then its id: the order decoders are
/// finished in, so which of two failures is reported does not change from one
/// render to the next.
type Key<'a> = (&'a TrackId, u64, &'a str);

/// A clip's decoder, and the last stretch it is heard in.
struct Open {
    decoder: AudioDecoder,
    last: usize,
}

impl<'r, 'a> Sources<'r, 'a> {
    pub(super) fn new(tools: &'r Tools, settings: &'r RenderSettings, plan: &'r Plan<'a>) -> Self {
        Self {
            tools,
            settings,
            plan,
            open: BTreeMap::new(),
        }
    }

    pub(super) const fn settings(&self) -> &'r RenderSettings {
        self.settings
    }

    /// The decoder `shot` reads stretch `index` from: the one already open for
    /// its clip, or a new one started where the clip enters this stretch, told
    /// how much it will be asked for across every stretch the clip stays in.
    pub(super) fn decoder(
        &mut self,
        index: usize,
        shot: &Shot<'a>,
        file: std::path::PathBuf,
    ) -> Result<&mut AudioDecoder, RenderError> {
        let clip = shot.clip.id.as_str();
        let key = (shot.track, shot.clip.start.get(), clip);
        if !self.open.contains_key(&key) {
            let (last, frames) = self.run(index, clip);
            let source = AudioSource {
                file,
                seek_seconds: self.plan.timeline_fps().seconds_at(shot.source_in),
                speed: shot.clip.speed,
                frames,
            };
            let decoder = AudioDecoder::start(self.tools, &source, self.settings)?;
            self.open.insert(key, Open { decoder, last });
        }
        Ok(&mut self.open.get_mut(&key).expect("opened above").decoder)
    }

    /// Finishes every decoder whose clip is not heard after stretch `index`,
    /// reporting the first that failed, in [`Key`] order.
    pub(super) fn close_after(&mut self, index: usize) -> Result<(), RenderError> {
        let done: Vec<Key<'a>> = self
            .open
            .iter()
            .filter(|(_, open)| open.last <= index)
            .map(|(key, _)| *key)
            .collect();
        for key in done {
            let open = self.open.remove(&key).expect("listed above");
            open.decoder.finish()?;
        }
        Ok(())
    }

    /// The last of the unbroken stretches from `index` that `clip` is heard
    /// in, and how many sample-frames they come to.
    fn run(&self, index: usize, clip: &str) -> (usize, u64) {
        let rate = self.settings.sample_rate.hz();
        let segments = self.plan.audio();
        let mut last = index;
        let mut frames = 0;
        for (at, segment) in segments.iter().enumerate().skip(index) {
            if !segment
                .layers
                .iter()
                .any(|shot| shot.clip.id.as_str() == clip)
            {
                break;
            }
            last = at;
            frames += self.plan.samples_of(segment, rate);
        }
        (last, frames)
    }
}
