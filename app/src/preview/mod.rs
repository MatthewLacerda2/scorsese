//! The preview: the picture at the playhead, and the transport under it.
//!
//! The point of having a window at all. `scorsese describe` says what the edit
//! claims; this is the first thing that shows whether it is right — which only
//! holds if the picture here is the picture a render delivers. It is: every
//! frame comes from [`scorsese_render::Renderer::still`], the render pipeline
//! with the encoder taken out, so there is no second way to draw the film and
//! nothing here can disagree with what ships.
//!
//! The playhead is **not** this module's. It lives in [`Editing`], above every
//! panel, because the scrubber under the preview and the line down the timeline
//! are one position with two views — two copies would disagree the first moment
//! someone dragged one.
//!
//! Sound is deliberately absent. Playing audio in step with the picture is a
//! second clock to keep in sync, and bolting it onto the first version of the
//! preview would tangle two hard things.

mod still;
mod transport;

use egui::{Panel, Ui};
use scorsese_core::{Fps, Frames};

use crate::editing::{Editing, length};
use crate::project::Open;
use still::Still;
use transport::Command;

/// The preview panel's own state.
#[derive(Default)]
pub(crate) struct Preview {
    /// The composited frame on screen, and what it took to get there.
    picture: Still,
    /// Set while the transport is running.
    playing: Option<Playing>,
}

/// A run of the transport: where the playhead was when play was pressed, and
/// when that was.
///
/// Time is kept from the press rather than accumulated frame by frame, so a
/// composite that takes longer than a frame costs *frames* and not *seconds*:
/// the playhead is always where the wall clock says it should be, and the
/// picture is whichever frame we managed to draw. Playback that slowed down
/// instead would be a preview lying about pacing, which is the one thing a
/// person watches a preview to judge.
struct Playing {
    /// Where the playhead was when it started.
    from: Frames,
    /// When that was.
    since: std::time::Instant,
}

impl Preview {
    /// Forgets the frame on screen and stops the transport, for when a
    /// different project is opened.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Forgets the composited frame, for when the document it was drawn from
    /// changed underneath it.
    ///
    /// Not [`Preview::reset`]: the playhead and the transport are untouched,
    /// because where someone is looking is not what changed. Only the picture
    /// is now a picture of a document that no longer exists.
    pub(crate) fn document_changed(&mut self) {
        self.picture.forget();
    }

    /// Draws the preview: the transport along the bottom, the picture above it.
    ///
    /// Declared in that order because that is what egui's layout wants — the
    /// panel takes its edge and the picture gets whatever is left, which is
    /// what makes the picture the thing that grows when the window does.
    pub(crate) fn show(&mut self, ui: &mut Ui, open: &Open, editing: &mut Editing) {
        let fps = open.project.timeline_fps;
        let last = transport::last_frame(length(&open.project));
        self.advance(ui, fps, last, editing);

        Panel::bottom("transport").show(ui, |ui| {
            match transport::show(ui, fps, editing.playhead.min(last), last) {
                Some(Command::Seek(frame)) => {
                    // Any seek stops playback, which is what every editor does:
                    // a step or a jump is someone taking hold of the playhead,
                    // and having it run away again is never what was meant.
                    editing.playhead = frame;
                    self.playing = None;
                }
                Some(Command::Toggle) => self.toggle(editing.playhead, last),
                None => {}
            }
        });

        // Clamped rather than refused: the timeline lets the playhead rest one
        // past the last frame — that is where the edit *ends* — and there is no
        // picture of an instant the film does not contain. Showing the last
        // frame is what a person means by parking at the end.
        self.picture.show(ui, open, editing.playhead.min(last));
    }

    /// Starts or stops the transport.
    ///
    /// Pressing play while parked at the end starts over, because the
    /// alternative is a play button that does nothing and gives no reason.
    fn toggle(&mut self, at: Frames, last: Frames) {
        if self.playing.take().is_some() {
            return;
        }
        self.playing = Some(Playing {
            from: if at >= last { Frames::ZERO } else { at },
            since: std::time::Instant::now(),
        });
    }

    /// Moves the playhead along the wall clock while the transport runs, and
    /// keeps the window repainting so that it does.
    fn advance(&mut self, ui: &Ui, fps: Fps, last: Frames, editing: &mut Editing) {
        let Some(playing) = &self.playing else {
            return;
        };
        let elapsed = playing.since.elapsed().as_secs_f64();
        let at = playing.from + fps.frames(elapsed);
        if at >= last {
            editing.playhead = last;
            self.playing = None;
            return;
        }
        editing.playhead = at;
        // egui only repaints when something happens, and time passing is not
        // something it counts. Without this the film would advance a frame per
        // mouse movement.
        ui.ctx().request_repaint();
    }
}
