//! Writing the frame under the playhead out as a PNG.
//!
//! The snapshot button every editor has, with the property that makes it worth
//! having: **what lands on disk is what the render would deliver**, not a
//! screengrab of the panel above it. The preview composites at a reduced raster
//! because it redraws while someone scrubs; a frame someone asked to keep is
//! composited again, at the raster a delivery is made at, through
//! [`scorsese_render::Renderer::still`] — the same call the panel uses and the
//! same one `scorsese still` uses.
//!
//! It is therefore slower than the preview by the ratio of the areas, and that
//! is the right trade for a thing a person does once and then looks at.
//!
//! **On a thread of its own** (#804). The composite is a render's, so it does
//! what a render does — including capturing a web page at the delivery raster
//! when `cache/` has no capture at that size yet, which is a minute of browser
//! for a heavy page, and the browser's download the first time. The preview's
//! own captures are at its reduced raster and do not count. So the window asks
//! where to put the file, hands the rest to a thread and keeps drawing, and
//! the line under the transport says it is saving until the file lands. The
//! frame is the document as it was when the button was pressed: an edit made
//! while it saves is not in it, the same as a render started before the edit.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use scorsese_core::{Frames, Project};
use scorsese_render::{RenderSettings, Renderer, Resolution, Tools, frames};

use crate::project::Open;

/// The raster a kept frame is composited at.
///
/// The default delivery size rather than the preview's, because the point of
/// keeping a frame is to look at it closely — and a still small enough to scrub
/// with is a still too small to judge a title by.
const DELIVERY: (u32, u32) = (1920, 1080);

/// What happened to the last frame someone asked to keep.
pub(super) enum Saved {
    /// It is being composited and written here, on a thread.
    Saving(PathBuf),
    /// It was written here.
    Wrote(PathBuf),
    /// It was not, and this is why. Said rather than swallowed: a button that
    /// silently does nothing is indistinguishable from a broken one.
    Failed(String),
}

/// The frames kept from the preview: the one being saved, if any, and what
/// became of the last.
#[derive(Default)]
pub(super) struct Keeper {
    /// What to say under the transport.
    last: Option<Saved>,
    /// The thread's answer, while one is saving.
    pending: Option<Receiver<Result<(), String>>>,
}

impl Keeper {
    /// Asks where to put the frame at `at`, then composites and writes it on a
    /// thread.
    ///
    /// A cancelled dialog changes nothing — someone who changed their mind
    /// knows they did. A press while a frame is still saving is not taken: the
    /// line already says one is, and two delivery-raster captures of the same
    /// page at once would be twice the browser for one file.
    pub(super) fn keep(&mut self, open: &Open, at: Frames) {
        if self.saving() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_title("Save this frame")
            .set_file_name(suggested(at))
            .add_filter("PNG image", &["png"])
            .save_file()
        else {
            return;
        };
        self.start(open.project.clone(), open.root.clone(), at, path);
    }

    /// Hands the composite and the write to a thread — split from
    /// [`Keeper::keep`] so it can be exercised without a dialog.
    fn start(&mut self, project: Project, root: PathBuf, at: Frames, path: PathBuf) {
        let (answer, pending) = mpsc::channel();
        let out = path.clone();
        std::thread::spawn(move || {
            // Sent into a channel nobody may be listening to any more — another
            // project opened meanwhile — which is fine: the file still lands.
            let _ = answer.send(write(&project, &root, at, &out));
        });
        self.last = Some(Saved::Saving(path));
        self.pending = Some(pending);
    }

    /// Takes the thread's answer when it has one. Called every frame.
    pub(super) fn poll(&mut self) {
        let Some(pending) = &self.pending else {
            return;
        };
        let outcome = match pending.try_recv() {
            Err(TryRecvError::Empty) => return,
            Ok(outcome) => outcome,
            Err(TryRecvError::Disconnected) => {
                Err("the save stopped before it finished".to_owned())
            }
        };
        self.pending = None;
        self.last = Some(match (outcome, self.last.take()) {
            (Ok(()), Some(Saved::Saving(path))) => Saved::Wrote(path),
            (Ok(()), _) => Saved::Failed("lost track of where it was saved".to_owned()),
            (Err(problem), _) => Saved::Failed(problem),
        });
    }

    /// Whether a frame is being saved right now.
    pub(super) fn saving(&self) -> bool {
        self.pending.is_some()
    }

    /// The line under the transport about the last frame kept.
    pub(super) fn note(&self, ui: &mut egui::Ui) {
        note(ui, self.last.as_ref());
    }
}

/// The composite and the write, with every failure worded for the strip under
/// the transport rather than for a terminal.
fn write(project: &Project, root: &Path, at: Frames, path: &Path) -> Result<(), String> {
    let tools = Tools::discover().map_err(|error| error.to_string())?;
    let raster = Resolution::new(DELIVERY.0, DELIVERY.1).map_err(|error| error.to_string())?;
    // The project's own grid, so the frame written is the frame the playhead is
    // standing on rather than the nearest one at some other rate.
    let settings = RenderSettings::new(raster, project.timeline_fps);
    let picture = Renderer::new(&tools, settings)
        .still(project, root, at)
        .map_err(|error| error.to_string())?;
    frames::write_png(&tools, path, &picture).map_err(|error| error.to_string())
}

/// What the dialog opens with: the frame number, zero-padded, so saving several
/// leaves a directory in playback order without anyone having to think about
/// it. The same shape `scorsese still` writes several frames under.
fn suggested(at: Frames) -> String {
    format!("frame-{:05}.png", at.get())
}

/// One line under the transport about the last frame kept, or nothing when no
/// one has kept any.
fn note(ui: &mut egui::Ui, saved: Option<&Saved>) {
    let text = match saved {
        None => return,
        Some(Saved::Saving(path)) => {
            egui::RichText::new(format!("saving {}…", path.display())).weak()
        }
        Some(Saved::Wrote(path)) => egui::RichText::new(format!("saved {}", path.display())).weak(),
        Some(Saved::Failed(problem)) => {
            egui::RichText::new(format!("not saved — {problem}")).color(ui.visuals().warn_fg_color)
        }
    };
    ui.label(text.small());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zero-padded and in playback order, which is the whole reason it is not
    /// simply `frame.png`: a directory of saved stills should sort the way the
    /// film runs.
    #[test]
    fn the_suggested_name_sorts_the_way_the_film_runs() {
        assert_eq!(suggested(Frames::ZERO), "frame-00000.png");
        assert_eq!(suggested(Frames(285)), "frame-00285.png");
        assert!(suggested(Frames(9)) < suggested(Frames(100)));
    }

    /// A keeper waiting on an answer it is handed by hand, so the outcome is
    /// the test's to choose.
    fn waiting(path: &str) -> (Keeper, mpsc::Sender<Result<(), String>>) {
        let (answer, pending) = mpsc::channel();
        let keeper = Keeper {
            last: Some(Saved::Saving(PathBuf::from(path))),
            pending: Some(pending),
        };
        (keeper, answer)
    }

    /// The window polls every frame; until the thread answers, it is saving and
    /// says so, and once it answers, it says where the file went.
    #[test]
    fn a_save_is_saving_until_its_thread_answers() {
        let (mut keeper, answer) = waiting("kept.png");
        keeper.poll();
        assert!(keeper.saving());
        assert!(matches!(&keeper.last, Some(Saved::Saving(path)) if path == Path::new("kept.png")));
        answer.send(Ok(())).expect("the keeper is listening");
        keeper.poll();
        assert!(!keeper.saving());
        assert!(matches!(&keeper.last, Some(Saved::Wrote(path)) if path == Path::new("kept.png")));
    }

    /// A failure is said, and so is a thread that died without answering —
    /// either way the line stops claiming it is still saving.
    #[test]
    fn a_failed_or_vanished_save_is_said() {
        let (mut keeper, answer) = waiting("kept.png");
        answer.send(Err("no room".to_owned())).expect("listening");
        keeper.poll();
        assert!(matches!(&keeper.last, Some(Saved::Failed(why)) if why == "no room"));

        let (mut keeper, answer) = waiting("kept.png");
        drop(answer);
        keeper.poll();
        assert!(!keeper.saving());
        assert!(matches!(keeper.last, Some(Saved::Failed(_))));
    }

    /// The real thread: `start` returns at once, saying it is saving, and the
    /// answer arrives later through `poll`. Into a folder that does not exist,
    /// so it ends in a failure whatever the machine has installed.
    #[test]
    fn start_returns_before_the_frame_is_written() {
        let project = Project::new("x", scorsese_core::Fps::THIRTY);
        let nowhere = std::env::temp_dir().join("scorsese-no-such-folder-804/kept.png");
        let mut keeper = Keeper::default();
        keeper.start(project, std::env::temp_dir(), Frames::ZERO, nowhere);
        assert!(keeper.saving());
        assert!(matches!(keeper.last, Some(Saved::Saving(_))));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while keeper.saving() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
            keeper.poll();
        }
        assert!(matches!(keeper.last, Some(Saved::Failed(_))));
    }

    /// A constant the renderer would refuse is a panic the first time anyone
    /// presses the button.
    #[test]
    fn the_delivery_raster_is_one_a_render_would_accept() {
        let raster = Resolution::new(DELIVERY.0, DELIVERY.1).expect("a legal raster");
        assert_eq!(raster.to_string(), "1920x1080");
    }
}
