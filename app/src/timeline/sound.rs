//! The waveform a sound clip carries: levels read once per file, off the UI
//! thread, and cut to the stretch of the file the clip plays.
//!
//! The same peaks the web draws (`scorsese_render::audio::peaks`, here computed
//! locally), kept under the project's rebuildable `cache/peaks/` and named for
//! the asset's hash, so a replaced file is read again and an unchanged one
//! never is. One worker, one file at a time, as the files panel's pictures are.
//! The slicing is `web/src/editor/timeline/peaks.ts`, bar for bar.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::Context;
use scorsese_render::Tools;
use scorsese_render::audio::{Peaks, peaks};

/// A bar of the waveform every this many pixels: two drawn, one a gap.
pub(super) const BAR_PX: f32 = 3.0;
/// The most bars one clip is drawn with, however long or close up.
const MOST_BARS: usize = 4096;

/// One file to read: where from, and where to keep the levels.
struct Job {
    source: PathBuf,
    out: PathBuf,
}

/// A finished job: the levels, or `None` for a file ffmpeg cannot read.
struct Done {
    out: PathBuf,
    peaks: Option<Peaks>,
}

enum Slot {
    Pending,
    Ready(Arc<Peaks>),
    Failed,
}

/// The worker and the levels it has delivered.
#[derive(Default)]
pub(super) struct Sounds {
    worker: Option<(Sender<Job>, Receiver<Done>)>,
    slots: HashMap<PathBuf, Slot>,
}

impl Sounds {
    /// Takes in whatever the worker has finished since the last frame.
    pub(super) fn poll(&mut self) {
        let Some((_, done)) = &self.worker else {
            return;
        };
        while let Ok(Done { out, peaks }) = done.try_recv() {
            let slot = peaks.map_or(Slot::Failed, |found| Slot::Ready(Arc::new(found)));
            self.slots.insert(out, slot);
        }
    }

    /// The levels kept at `out`, asking for them to be read if nobody has yet.
    pub(super) fn get(&mut self, ctx: &Context, source: &Path, out: &Path) -> Option<Arc<Peaks>> {
        if !self.slots.contains_key(out) {
            let (jobs, _) = self.worker.get_or_insert_with(|| spawn(ctx.clone()));
            let queued = jobs
                .send(Job {
                    source: source.to_path_buf(),
                    out: out.to_path_buf(),
                })
                .is_ok();
            self.slots.insert(
                out.to_path_buf(),
                if queued { Slot::Pending } else { Slot::Failed },
            );
        }
        match self.slots.get(out) {
            Some(Slot::Ready(found)) => Some(Arc::clone(found)),
            _ => None,
        }
    }
}

fn spawn(ctx: Context) -> (Sender<Job>, Receiver<Done>) {
    let (jobs, queue) = channel::<Job>();
    let (done, results) = channel::<Done>();
    std::thread::spawn(move || {
        let tools = Tools::discover().ok();
        for Job { source, out } in queue {
            let found = std::fs::read(&out)
                .ok()
                .and_then(|bytes| Peaks::from_bytes(&bytes))
                .or_else(|| {
                    let read = peaks(tools.as_ref()?, &source).ok()?;
                    if let Some(folder) = out.parent() {
                        let _ = std::fs::create_dir_all(folder);
                    }
                    let _ = std::fs::write(&out, read.to_bytes());
                    Some(read)
                });
            if done.send(Done { out, peaks: found }).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    });
    (jobs, results)
}

/// How many bars `length` seconds of sound, drawn `width` pixels wide, get: one
/// every few pixels, never more than the levels the slice holds, and never more
/// than [`MOST_BARS`]. At least one.
pub(super) fn bars_of(per_second: f32, length: f64, width: f32) -> usize {
    let levels = (length * f64::from(per_second) - 1e-9).ceil().max(0.0) as usize;
    let wide = (width / BAR_PX).floor() as usize;
    wide.min(levels).clamp(1, MOST_BARS)
}

/// `bars` levels, 0–1, for the `length` seconds of the sound from `from`: each
/// the loudest level its stretch touches, silence past the end of the file.
pub(super) fn slice_of(found: &Peaks, from: f64, length: f64, bars: usize) -> Vec<f32> {
    let each = length / bars as f64;
    let rate = f64::from(found.per_second);
    (0..bars)
        .map(|bar| {
            let first = ((from + bar as f64 * each) * rate + 1e-9).floor();
            let end = (first + 1.0).max(((from + (bar + 1) as f64 * each) * rate - 1e-9).ceil());
            let first = first.max(0.0) as usize;
            let end = (end.max(0.0) as usize).min(found.levels.len());
            let loudest = found
                .levels
                .get(first..end)
                .map_or(0, |run| run.iter().copied().max().unwrap_or(0));
            f32::from(loudest) / 255.0
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peaks_of(levels: &[u8]) -> Peaks {
        Peaks {
            per_second: 10.0,
            levels: levels.to_vec(),
        }
    }

    #[test]
    fn a_bar_is_taken_every_few_pixels_and_never_finer_than_the_levels() {
        assert_eq!(bars_of(10.0, 10.0, 300.0), 100);
        assert_eq!(bars_of(10.0, 2.0, 3000.0), 20, "no finer than the levels");
        assert_eq!(bars_of(10.0, 600.0, 90_000.0), MOST_BARS);
        assert_eq!(bars_of(10.0, 5.0, 1.0), 1, "at least one");
    }

    #[test]
    fn each_bar_is_the_loudest_level_its_stretch_touches() {
        let found = peaks_of(&[0, 51, 255, 0, 102, 0]);
        // Two bars over the first six levels: three levels each.
        let bars = slice_of(&found, 0.0, 0.6, 2);
        assert_eq!(bars, [1.0, 102.0 / 255.0]);
    }

    #[test]
    fn a_trimmed_clip_reads_from_its_in_point() {
        let found = peaks_of(&[255, 255, 0, 0]);
        let bars = slice_of(&found, 0.2, 0.2, 2);
        assert_eq!(bars, [0.0, 0.0], "the loud start was trimmed away");
    }

    #[test]
    fn past_the_end_of_the_file_is_silence() {
        let found = peaks_of(&[255, 255]);
        let bars = slice_of(&found, 0.1, 0.4, 4);
        assert_eq!(bars[0], 1.0);
        assert_eq!(&bars[1..], [0.0, 0.0, 0.0]);
    }
}
