//! The preview: the picture at the playhead, and the transport under it.
//!
//! The point of having a window at all. `scorsese describe` says what the edit
//! claims; this is the first thing that shows whether it is right — which only
//! holds if the picture here is the picture a render delivers. It is: every
//! frame comes from [`scorsese_render::Renderer::still`], the render pipeline
//! with the encoder taken out, so there is no second way to draw the film and
//! nothing here can disagree with what ships.
//!
//! **At full quality, pixel for pixel.** Playing a cut in real time is how
//! pacing is judged, and a 1080p frame decoded from a 4K screen recording is
//! slow to get, so [`quality`] trades pixels for speed (#542): half or a
//! quarter of the delivery raster, reading [`proxies`] of heavy videos where
//! they are made. That is still the render's frame — the same compositor, the
//! same layout — only smaller, and the line under the transport always says
//! which mode the picture is in.
//!
//! The playhead is **not** this module's. It lives in [`Editing`], above every
//! panel, because the scrubber under the preview and the line down the timeline
//! are one position with two views — two copies would disagree the first moment
//! someone dragged one.
//!
//! [`save`] is that same frame kept: the button under the picture writes the
//! instant under the playhead out as a PNG, composited again at delivery
//! resolution rather than lifted off the panel — a screengrab of a preview pane
//! is not what the film looks like. On a thread, so a page that has to be
//! captured at that size first never freezes the window (#804).
//!
//! Sound plays with it, and the same rule holds: every sample comes from the
//! renderer's own mixer, so what you hear is what ships. See [`sound`] for how
//! it is made and, more importantly, for which of the two is the clock — the
//! answer is not the one this module started with.

pub(crate) mod pages;
mod proxies;
mod quality;
mod save;
mod sound;
mod still;
mod transport;

use egui::{Panel, Ui};
use scorsese_core::{Fps, Frames, Project};
use scorsese_render::Quality;

use crate::editing::{Editing, length};
use crate::project::Open;
use still::Still;
use transport::Command;

/// The last frame the playhead may stand on in `project`.
///
/// Published from here because the transport is what decides where "the end"
/// is — `length` counts frames and the last one is below it — and the keyboard
/// needs the same answer. Two spellings of it would be two ends of the same
/// film.
pub(crate) fn last_frame(project: &Project) -> Frames {
    transport::last_frame(length(project))
}

/// The preview panel's own state.
#[derive(Default)]
pub(crate) struct Preview {
    /// The composited frame on screen, and what it took to get there.
    picture: Still,
    /// Set while the transport is running.
    playing: Option<Playing>,
    /// The frame being kept, if one is, and what became of the last. Held so
    /// it can be said under the transport — a save that reported nothing would
    /// be indistinguishable from a button that does nothing.
    saved: save::Keeper,
    /// How much of the delivery raster the picture draws. For the session:
    /// it outlives opening another project, and is never written to one.
    quality: Quality,
    /// The open project's proxies, made in the background at a quality that
    /// reads them.
    proxies: proxies::Maker,
    /// The open project's web pages, captured in the background at the
    /// preview's raster (#776).
    pages: pages::Capturer,
}

/// A run of the transport: where the playhead was when play was pressed, when
/// that was, and the sound that takes over timing it.
///
/// Time is kept from the press rather than accumulated frame by frame, so a
/// composite that takes longer than a frame costs *frames* and not *seconds*:
/// the playhead is where the clock says it should be, and the picture is
/// whichever frame we managed to draw. Playback that slowed down instead would
/// be a preview lying about pacing, which is the one thing a person watches a
/// preview to judge.
///
/// **Which clock, though, is the sound's to say.** A dropped video frame is
/// invisible; a dropped sample is a click and a stretched one is a pitch
/// change, so audio cannot be made to follow anything. Once the mix is
/// playing, its position through the buffer *is* the playhead. The wall clock
/// stays as the fallback for the moment before the mix is ready, for a film
/// with nothing audible in it, and for a machine with no sound card — which is
/// exactly what playback did before there was any sound at all.
struct Playing {
    /// Where the playhead was when it started.
    from: Frames,
    /// When that was — the fallback clock.
    since: std::time::Instant,
    /// The soundtrack, which becomes the clock as soon as it is playing.
    sound: sound::Sound,
}

impl Preview {
    /// Forgets the frame on screen and stops the transport, for when a
    /// different project is opened.
    pub(crate) fn reset(&mut self) {
        *self = Self {
            quality: self.quality,
            ..Self::default()
        };
    }

    /// Forgets the composited frame, for when the document it was drawn from
    /// changed underneath it.
    ///
    /// Not [`Preview::reset`]: the playhead and the transport are untouched,
    /// because where someone is looking is not what changed. Only the picture
    /// is now a picture of a document that no longer exists.
    pub(crate) fn document_changed(&mut self) {
        self.picture.forget();
        self.proxies.document_changed();
        self.pages.document_changed();
    }

    /// Forgets the composited frame and looks for pages to capture again, for
    /// when a page's file changed and the document did not (#808).
    ///
    /// Narrower than [`Preview::document_changed`]: no asset changed, so there
    /// is no proxy to look for. The frame is forgotten because it may show the
    /// page as it was, or its slug card where a capture of it now is not.
    pub(crate) fn pages_changed(&mut self) {
        self.picture.forget();
        self.pages.document_changed();
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
        self.follow_proxies(ui, open);
        self.follow_pages(ui, open);
        self.follow_save(ui);

        let silent = self
            .playing
            .as_ref()
            .and_then(|playing| playing.sound.trouble())
            .map(str::to_owned);
        Panel::bottom("transport").show(ui, |ui| {
            match transport::show(ui, fps, editing.playhead.min(last), last, silent.as_deref()) {
                Some(Command::Seek(frame)) => {
                    // Any seek stops playback, which is what every editor does:
                    // a step or a jump is someone taking hold of the playhead,
                    // and having it run away again is never what was meant.
                    editing.playhead = frame;
                    self.playing = None;
                }
                Some(Command::Toggle) => self.toggle(open, editing.playhead, last),
                // Composited again rather than taken off the panel above: the
                // preview draws at a reduced raster, and a kept frame is the
                // one a render would deliver.
                Some(Command::Keep) => self.saved.keep(open, editing.playhead.min(last)),
                None => {}
            }
            self.saved.note(ui);
            let making = [self.proxies.status(), self.pages.status()]
                .into_iter()
                .flatten()
                .reduce(|one, other| format!("{one} · {other}"));
            if let Some(chosen) = quality::show(ui, self.quality, making.as_deref()) {
                self.quality = chosen;
            }
        });

        // Clamped rather than refused: the timeline lets the playhead rest one
        // past the last frame — that is where the edit *ends* — and there is no
        // picture of an instant the film does not contain. Showing the last
        // frame is what a person means by parking at the end.
        let at = editing.playhead.min(last);
        self.picture
            .show(ui, open, at, self.quality, self.pages.chrome());
        // Not while playing: the playhead has moved on before a capture of
        // this instant could land, and the whole page is on its way anyway.
        if self.picture.uncaptured() && self.playing.is_none() {
            self.pages.playhead(open, at, self.quality);
            // Nothing else would look for it landing until the pointer moves.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// Takes a kept frame's outcome when its thread has one — the window never
    /// waits for a save (#804), so nothing else would notice it finishing.
    fn follow_save(&mut self, ui: &Ui) {
        self.saved.poll();
        if self.saved.saving() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    /// Captures the pages the picture shows, and draws it again as each lands
    /// — the picture itself never waits for a page (#776).
    fn follow_pages(&mut self, ui: &Ui, open: &Open) {
        self.pages.ensure(open, self.quality);
        if self.pages.landed() {
            self.picture.forget();
        }
        if self.pages.running() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// Makes the proxies a reduced quality reads, and draws the picture again
    /// from each one as it lands.
    fn follow_proxies(&mut self, ui: &Ui, open: &Open) {
        if !self.quality.uses_proxies() {
            return;
        }
        self.proxies.ensure(open);
        if self.proxies.landed() {
            self.picture.forget();
        }
        // Nothing else repaints while a transcode runs in another thread, and
        // the line saying how far it got should move.
        if self.proxies.running() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// Starts or stops the transport, for the key that does what the button
    /// under the picture does.
    pub(crate) fn toggle_playback(&mut self, open: &Open, at: Frames, last: Frames) {
        self.toggle(open, at, last);
    }

    /// Stops it, for anything that takes hold of the playhead.
    pub(crate) fn stop(&mut self) {
        self.playing = None;
    }

    /// Starts or stops the transport.
    ///
    /// Pressing play while parked at the end starts over, because the
    /// alternative is a play button that does nothing and gives no reason.
    /// Stopping drops the `Playing`, and with it the sound — which is the
    /// whole of "stopping must not leave a voice hanging". There is one owner
    /// of the stream and letting go of it is the stop button.
    fn toggle(&mut self, open: &Open, at: Frames, last: Frames) {
        if self.playing.take().is_some() {
            return;
        }
        let from = if at >= last { Frames::ZERO } else { at };
        self.playing = Some(Playing {
            from,
            since: std::time::Instant::now(),
            sound: sound::Sound::start(&open.project, &open.root, from),
        });
    }

    /// Moves the playhead along whichever clock is running the transport, and
    /// keeps the window repainting so that it does.
    fn advance(&mut self, ui: &Ui, fps: Fps, last: Frames, editing: &mut Editing) {
        let Some(playing) = &mut self.playing else {
            return;
        };
        // Takes the mix from the worker the moment it is ready, and starts it.
        playing.sound.poll();
        // Sound is the clock when there is sound; the wall clock is what is
        // left when there is not.
        let at = playing
            .sound
            .at(fps)
            .unwrap_or_else(|| playing.from + fps.frames(playing.since.elapsed().as_secs_f64()));
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
