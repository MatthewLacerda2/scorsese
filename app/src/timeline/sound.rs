//! The waveform a sound clip carries: levels read once per file, off the UI
//! thread, and cut to the stretch of the file the clip plays.
//!
//! The same peaks the web draws (`scorsese_render::audio::peaks`, here computed
//! locally), kept under the project's rebuildable `cache/peaks/` and named for
//! the asset's hash, so a replaced file is read again and an unchanged one
//! never is. One worker, one file at a time, as the files panel's pictures are.
//! The slicing is `web/src/editor/timeline/peaks.ts`, bar for bar.
//!
//! Kept as the files panel's pictures are, never through a link (#1737): the
//! folder is made through [`scorsese_core::write::in_cache`], a link at the
//! file's name is read again rather than believed, and the levels are written
//! through [`scorsese_core::write::atomically`], which replaces it.

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

/// The folder below `cache/` the levels are kept in.
pub(super) const FOLDER: &str = "peaks";

/// One file to read: where from, the project the levels are kept in, and
/// where in it.
struct Job {
    source: PathBuf,
    root: PathBuf,
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

    /// The levels kept at `out` under the project at `root`, asking for them
    /// to be read if nobody has yet.
    pub(super) fn get(
        &mut self,
        ctx: &Context,
        source: &Path,
        root: &Path,
        out: &Path,
    ) -> Option<Arc<Peaks>> {
        if !self.slots.contains_key(out) {
            let (jobs, _) = self.worker.get_or_insert_with(|| spawn(ctx.clone()));
            let queued = jobs
                .send(Job {
                    source: source.to_path_buf(),
                    root: root.to_path_buf(),
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
        for job in queue {
            let found = found(tools.as_ref(), &job);
            let out = job.out;
            if done.send(Done { out, peaks: found }).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    });
    (jobs, results)
}

/// `job`'s levels: kept from before, or read now and kept — the folder made,
/// and the file written, never through a link (the module's doc). Levels that
/// cannot be kept are still drawn: a refused folder is neither read from nor
/// written to, and the sound is read afresh.
fn found(tools: Option<&Tools>, job: &Job) -> Option<Peaks> {
    let folder = scorsese_core::write::in_cache(&job.root, Path::new(FOLDER));
    if let Err(error) = &folder {
        eprintln!("no waveforms are kept for this project: {error}");
    }
    let kept = folder.is_ok() && crate::files::thumbs::is_kept(&job.out);
    let read_back = kept
        .then(|| std::fs::read(&job.out).ok())
        .flatten()
        .and_then(|bytes| Peaks::from_bytes(&bytes));
    read_back.or_else(|| {
        let read = peaks(tools?, &job.source).ok()?;
        if folder.is_ok()
            && let Err(error) = scorsese_core::write::atomically(&job.out, read.to_bytes())
        {
            eprintln!(
                "the waveform of {} is not kept: {error}",
                job.source.display()
            );
        }
        Some(read)
    })
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

    /// A sound to read, drawn by ffmpeg, and the job that reads it into the
    /// project at `root`.
    fn job(tools: &Tools, root: &Path) -> Job {
        let source = root.join("tone.wav");
        let made = tools
            .ffmpeg()
            .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi"])
            .args(["-i", "sine=duration=0.5"])
            .arg(&source)
            .status()
            .expect("ffmpeg runs");
        assert!(made.success(), "the fixture sound");
        Job {
            source,
            root: root.to_path_buf(),
            out: root.join("cache").join(FOLDER).join("tone-aaaa.bin"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_cache_folder_is_drawn_from_but_nothing_lands_where_it_leads() {
        let tools = Tools::discover().expect("ffmpeg");
        for linked in ["cache", "cache/peaks"] {
            let (root, outside) = crate::files::thumbs::tests::planted("peaks-linked-folder");
            let job = job(&tools, &root);
            let link = root.join(linked);
            std::fs::create_dir_all(link.parent().expect("a parent")).expect("cache/");
            std::os::unix::fs::symlink(&outside, &link).expect("a link");

            assert!(found(Some(&tools), &job).is_some(), "{linked}: still drawn");
            let landed = std::fs::read_dir(&outside).expect("outside").count();
            assert_eq!(landed, 0, "{linked}: written where the link leads");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_at_the_levels_name_is_replaced_never_believed_or_written_through() {
        let tools = Tools::discover().expect("ffmpeg");
        let (root, outside) = crate::files::thumbs::tests::planted("peaks-linked-file");
        let job = job(&tools, &root);
        let planted = Peaks {
            per_second: 1.0,
            levels: vec![7],
        };
        let victim = outside.join("victim.bin");
        std::fs::write(&victim, planted.to_bytes()).expect("a victim");
        std::fs::create_dir_all(job.out.parent().expect("a parent")).expect("cache/peaks");
        std::os::unix::fs::symlink(&victim, &job.out).expect("a link");

        let read = found(Some(&tools), &job).expect("the levels are read");
        assert_ne!(read.levels, planted.levels, "the link was believed");
        assert_eq!(
            std::fs::read(&victim).expect("the victim"),
            planted.to_bytes()
        );
        let landed = std::fs::symlink_metadata(&job.out).expect("the file");
        assert!(!landed.is_symlink(), "the link was not replaced");
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
