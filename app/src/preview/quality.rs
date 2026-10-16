//! The preview quality control, and the line saying which mode the picture is
//! in (#542).
//!
//! Three steps — full, half, quarter of the project's delivery raster, in its
//! own shape — which mean the same here as in the web editor. Kept for the
//! session rather than written anywhere: it is about this machine and this
//! moment, not about the edit, so it has no business in `project.json`.
//!
//! **The picture is only the render's picture at full quality**, and the
//! module doc above this one promises exactly that. So the mode is always said,
//! quietly, beside the control: at a reduced quality what is on screen is the
//! same frame drawn with fewer pixels, from proxies where they are made.

use egui::{RichText, Ui};
use scorsese_core::Ratio;
use scorsese_render::{Budget, Quality, Resolution};

/// The raster a preview at `quality` of a project shaped `aspect` is
/// composited at: a fraction of the size a render of it delivers by default,
/// so the picture is drawn in the project's own shape (#1278).
pub(super) fn raster(quality: Quality, aspect: Ratio) -> Resolution {
    quality.raster(Resolution::framed(aspect, Budget::Delivery))
}

/// Draws the control and the line under it; the quality chosen, if one was.
pub(super) fn show(
    ui: &mut Ui,
    quality: Quality,
    aspect: Ratio,
    making: Option<&str>,
) -> Option<Quality> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preview").small().weak());
        for step in Quality::ALL {
            if ui
                .selectable_label(step == quality, step.label())
                .on_hover_text(hint(step))
                .clicked()
                && step != quality
            {
                chosen = Some(step);
            }
        }
        ui.label(RichText::new(mode(quality, aspect, making)).small().weak());
    });
    chosen
}

/// What the picture is, at `quality`, in the words under the control.
fn mode(quality: Quality, aspect: Ratio, making: Option<&str>) -> String {
    let raster = raster(quality, aspect);
    let said = if quality.uses_proxies() {
        format!("{raster}, from proxies where made")
    } else {
        format!("{raster}, the render's own picture")
    };
    match making {
        Some(making) => format!("{said} — {making}"),
        None => said,
    }
}

/// What a step does, for the pointer resting on it.
fn hint(quality: Quality) -> &'static str {
    match quality {
        Quality::Full => "Every pixel, from the original files: exactly what a render delivers",
        Quality::Half => "Half the size each way: smoother playback, proxies of heavy videos",
        Quality::Quarter => "A quarter the size each way: the smoothest, proxies too",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every raster a quality asks for is one the encoder would accept, and a
    /// constant the renderer refused would be a failure on the first frame.
    #[test]
    fn every_quality_draws_at_a_raster_a_render_would_accept() {
        let rasters = |aspect| -> Vec<String> {
            Quality::ALL
                .into_iter()
                .map(|quality| raster(quality, aspect).to_string())
                .collect()
        };
        assert_eq!(rasters(Ratio::WIDE), ["1920x1080", "960x540", "480x270"]);
        assert_eq!(rasters(Ratio::TALL), ["1080x1920", "540x960", "270x480"]);
    }

    #[test]
    fn the_mode_says_whether_the_picture_is_the_renders_own() {
        assert!(mode(Quality::Full, Ratio::WIDE, None).contains("the render's own picture"));
        assert!(mode(Quality::Half, Ratio::WIDE, None).contains("proxies"));
        let making = mode(
            Quality::Quarter,
            Ratio::TALL,
            Some("making proxies: 1 of 2"),
        );
        assert!(making.ends_with("making proxies: 1 of 2"), "{making}");
    }
}
