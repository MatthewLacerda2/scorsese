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

use std::path::PathBuf;

use scorsese_core::Frames;
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
    /// It was written here.
    Wrote(PathBuf),
    /// It was not, and this is why. Said rather than swallowed: a button that
    /// silently does nothing is indistinguishable from a broken one.
    Failed(String),
}

/// Asks where to put it, composites, and writes it.
///
/// `None` means the dialog was cancelled, which is not an outcome worth
/// reporting — someone who changed their mind knows they did.
pub(super) fn frame(open: &Open, at: Frames) -> Option<Saved> {
    let path = rfd::FileDialog::new()
        .set_title("Save this frame")
        .set_file_name(suggested(at))
        .add_filter("PNG image", &["png"])
        .save_file()?;
    Some(match write(open, at, &path) {
        Ok(()) => Saved::Wrote(path),
        Err(problem) => Saved::Failed(problem),
    })
}

/// The composite and the write, with every failure worded for the strip under
/// the transport rather than for a terminal.
fn write(open: &Open, at: Frames, path: &std::path::Path) -> Result<(), String> {
    let tools = Tools::discover().map_err(|error| error.to_string())?;
    let raster = Resolution::new(DELIVERY.0, DELIVERY.1).map_err(|error| error.to_string())?;
    // The project's own grid, so the frame written is the frame the playhead is
    // standing on rather than the nearest one at some other rate.
    let settings = RenderSettings::new(raster, open.project.timeline_fps);
    let picture = Renderer::new(&tools, settings)
        .still(&open.project, &open.root, at)
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
pub(super) fn note(ui: &mut egui::Ui, saved: Option<&Saved>) {
    let text = match saved {
        None => return,
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

    /// A constant the renderer would refuse is a panic the first time anyone
    /// presses the button.
    #[test]
    fn the_delivery_raster_is_one_a_render_would_accept() {
        let raster = Resolution::new(DELIVERY.0, DELIVERY.1).expect("a legal raster");
        assert_eq!(raster.to_string(), "1920x1080");
    }
}
