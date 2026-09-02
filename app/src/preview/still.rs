//! The picture: one composited frame, cached, drawn as a texture.
//!
//! **Cached, and that is not an optimisation.** Compositing one frame spawns an
//! ffmpeg per layer that has a source — tens of milliseconds at best — and egui
//! repaints for reasons that have nothing to do with the picture: a pointer
//! moving, a window resizing, a tooltip fading. Recompositing on every repaint
//! would make the whole window as slow as a decode. So the frame is kept, and
//! it is redrawn only when the instant asked for or the raster it was drawn at
//! changes.
//!
//! It is deliberately *one* frame and not a ring of them. Reading ahead so that
//! playback is smooth is a real piece of work with a real design behind it —
//! decode threads, a buffer, a policy for throwing it away — and it should be
//! built from a measurement of this, not instead of it.

use egui::{
    Align2, Color32, FontId, Image, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui,
};
use scorsese_core::Frames;
use scorsese_render::{RenderSettings, Renderer, Resolution, Tools};

use crate::project::Open;
use crate::theme::{marks, palette};

/// How long the arm of a corner mark is.
///
/// Long enough to read as a right angle and short enough that four of them
/// never join up into a box. See [`marks::corners`] for why a frame around a
/// picture is four marks rather than a rectangle.
const ARM: f32 = 18.0;
/// How far off the picture the marks stand, where there is room for them to.
const GAP: f32 = 5.0;

/// The raster the preview composites at.
///
/// Reduced on purpose: a preview is for judging pacing and framing, not for
/// pixel-peeping, and the cost of a frame is roughly its area. 16:9 because the
/// document does not record an aspect — a render's raster is chosen per render
/// and never stored — so what the window can honestly show is the shape the
/// default delivery has.
const RASTER: (u32, u32) = (640, 360);

/// The frame on screen, and what stopped there being one.
#[derive(Default)]
pub(super) struct Still {
    /// The instant the picture — or the problem — below is the answer to.
    ///
    /// Remembered whichever way the attempt went, which is what stops a failure
    /// being retried on every repaint: an edit with no video track in it would
    /// otherwise spawn ffmpeg sixty times a second to fail sixty times.
    asked: Option<Frames>,
    /// The frame on the GPU, if the last attempt produced one. Its size is the
    /// raster it was composited at, so the two cannot be stored apart and
    /// disagree.
    texture: Option<TextureHandle>,
    /// Why it did not, in the words to put where the picture would be.
    problem: Option<String>,
    /// ffmpeg, looked for once and then remembered — failure included. A
    /// machine without ffmpeg does not grow one mid-session, and checking on
    /// every scrubbed frame would spawn two processes to learn that again.
    tools: Option<Result<Tools, String>>,
}

impl Still {
    /// Draws the picture at `at`, compositing it first if it is not the one
    /// already held.
    pub(super) fn show(&mut self, ui: &mut Ui, open: &Open, at: Frames) {
        let raster = raster();
        let wanted = [raster.width() as usize, raster.height() as usize];
        let stale = self.asked != Some(at)
            || self
                .texture
                .as_ref()
                .is_some_and(|texture| texture.size() != wanted);
        if stale {
            self.recompose(ui, open, at, raster);
        }
        self.paint(ui);
    }

    /// Asks the renderer for one frame and hands it to the GPU.
    fn recompose(&mut self, ui: &Ui, open: &Open, at: Frames, raster: Resolution) {
        self.asked = Some(at);
        // Cloned out of `self` before anything else is touched: the discovery
        // borrows the whole struct, and the failure has to be written back into
        // it. Both halves are cheap — a `Tools` is two paths.
        let tools = self
            .tools
            .get_or_insert_with(|| Tools::discover().map_err(|error| error.to_string()))
            .clone();
        let tools = match tools {
            Ok(tools) => tools,
            Err(problem) => {
                self.failed(problem);
                return;
            }
        };

        // The project's own grid, so no conform happens and the frame shown is
        // the frame asked for rather than the nearest one at some other rate.
        let settings = RenderSettings::new(raster, open.project.timeline_fps);
        let frame = match Renderer::new(&tools, settings).still(&open.project, &open.root, at) {
            Ok(frame) => frame,
            Err(problem) => {
                self.failed(problem.to_string());
                return;
            }
        };

        let image = egui::ColorImage::from_rgba_unmultiplied(
            [raster.width() as usize, raster.height() as usize],
            frame.bytes(),
        );
        self.problem = None;
        match &mut self.texture {
            // Overwritten rather than re-allocated: scrubbing replaces this
            // texture many times a second, and a fresh allocation each time is
            // churn the driver has to clean up behind us.
            Some(texture) => texture.set(image, TextureOptions::LINEAR),
            None => {
                self.texture = Some(ui.ctx().load_texture(
                    "preview",
                    image,
                    TextureOptions::LINEAR,
                ));
            }
        }
    }

    /// Forgets which instant the held frame answers, so the next repaint
    /// composites again. The texture is kept until there is one to replace it
    /// with — dropping it here would blink the panel to black on every reload.
    pub(super) fn forget(&mut self) {
        self.asked = None;
    }

    /// Records why there is no picture, and drops the one that is no longer
    /// true. A stale frame left on screen under a new playhead would be the
    /// preview lying, which is worse than the preview being empty.
    fn failed(&mut self, problem: String) {
        self.texture = None;
        self.problem = Some(problem);
    }

    /// Puts the frame on screen, letterboxed into whatever room the panel has.
    fn paint(&self, ui: &mut Ui) {
        let area = ui.available_rect_before_wrap();
        ui.allocate_rect(area, Sense::hover());
        let painter = ui.painter_at(area);
        // Black behind it, always: the picture keeps its own shape rather than
        // stretching to the panel, and what surrounds a picture is black.
        painter.rect_filled(area, 0.0, Color32::BLACK);

        let Some(texture) = &self.texture else {
            painter.text(
                area.center(),
                Align2::CENTER_CENTER,
                self.problem.as_deref().unwrap_or("nothing to show here"),
                FontId::proportional(13.0),
                palette::FAINT,
            );
            return;
        };
        let size = texture.size_vec2();
        let frame = fitted(area, size.x / size.y);
        Image::new((texture.id(), size)).paint_at(ui, frame);
        // Outside the picture where there is room and against its edge where
        // there is not — which is nearly always, in one direction or the other,
        // because a picture letterboxed into a panel touches two of its sides
        // by definition. Clipped to the panel rather than allowed to run past
        // it: a corner mark with one arm missing is worse than no mark, and a
        // half-drawn one is exactly what an unclamped `expand` produces on the
        // axis the picture is already filling.
        //
        // The marks are what makes the boundary between the picture and the
        // matte visible when the frame itself is nearly black, which is most of
        // the frames in most of the films anybody cuts.
        marks::corners(
            &painter,
            frame.expand(GAP).intersect(area),
            ARM,
            Stroke::new(1.0, palette::ACCENT.gamma_multiply(0.55)),
        );
    }
}

/// The raster every preview frame is composited at.
fn raster() -> Resolution {
    Resolution::new(RASTER.0, RASTER.1).expect("the preview raster is a legal one")
}

/// The largest rectangle of `aspect` that fits inside `area`, centred on it —
/// which is what "show the picture without distorting it" means.
fn fitted(area: Rect, aspect: f32) -> Rect {
    let width = area.width().min(area.height() * aspect);
    Rect::from_center_size(area.center(), egui::vec2(width, width / aspect))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    /// Letterboxing, both ways round: a wide picture in a tall panel gains room
    /// above and below, a tall one in a wide panel gains it at the sides, and
    /// neither is ever stretched.
    #[test]
    fn a_picture_keeps_its_shape_whatever_shape_the_panel_is() {
        let widescreen = 16.0 / 9.0;
        for panel in [vec2(800.0, 200.0), vec2(200.0, 800.0), vec2(320.0, 180.0)] {
            let area = Rect::from_min_size(pos2(10.0, 20.0), panel);
            let fit = fitted(area, widescreen);
            assert!(
                (fit.aspect_ratio() - widescreen).abs() < 1e-3,
                "a {panel:?} panel distorted the picture to {}",
                fit.aspect_ratio()
            );
            assert!(fit.width() <= area.width() + 1e-3 && fit.height() <= area.height() + 1e-3);
            assert!((fit.center() - area.center()).length() < 1e-3, "off centre");
        }
    }

    /// The raster is a constant, and a constant the encoder would refuse is a
    /// panic on the first frame anyone previews.
    #[test]
    fn the_preview_raster_is_one_a_render_would_accept() {
        assert_eq!(raster().to_string(), "640x360");
    }
}
