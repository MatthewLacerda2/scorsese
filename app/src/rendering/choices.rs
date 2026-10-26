//! What a render is asked to make: the plain settings, and their defaults.
//!
//! Four choices and no more — size, frame rate, kind of file, loudness — each with a
//! default good enough that pressing Render without touching any of them gives
//! the file most people want: a 1080p mp4 at the edit's own rate, in the
//! project's own shape (#1278) — 1920 × 1080 for 16:9, 1080 × 1920 for 9:16. Bitrates,
//! codecs and sample rates stay at the renderer's defaults; they are how the
//! file is made, not what it is, and the CLI is where somebody who cares about
//! them already is.

use egui::{ComboBox, Ui};
use scorsese_core::frame::{DELIVERY_LINES, even};
use scorsese_core::{Fps, Ratio};
use scorsese_render::{Container, LoudnessTarget, OutputFormat, RenderSettings, Resolution};

/// The sizes on offer, as `(name, lines on the frame's shorter side)`, each in
/// the project's shape: its default delivery first (`Ratio::delivery`). The
/// shape itself is the project's to change (`crate::framing`), not a size's.
const SIZES: [(&str, u32); 3] = [("1080p", DELIVERY_LINES), ("720p", 720), ("4K", 2160)];

/// The rates on offer besides the edit's own.
const RATES: [Fps; 4] = [Fps::FILM, Fps::PAL, Fps::THIRTY, Fps::SIXTY];

/// Where the soundtrack is headed, as the web's menu words it: the loudness a
/// destination plays back at, or the mix as balanced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Loudness {
    /// The mix as balanced, conformed to nothing.
    Mixed,
    /// Streaming platforms and social feeds, about −14 LUFS.
    Streaming,
    /// Podcast apps, about −16 LUFS.
    Podcast,
}

impl Loudness {
    const ALL: [Self; 3] = [Self::Mixed, Self::Streaming, Self::Podcast];

    /// The integrated loudness to deliver at, in LUFS; `None` leaves the mix.
    fn lufs(self) -> Option<f64> {
        match self {
            Self::Mixed => None,
            Self::Streaming => Some(-14.0),
            Self::Podcast => Some(-16.0),
        }
    }

    /// The entry as the menu says it — the web's own words.
    fn words(self) -> &'static str {
        match self {
            Self::Mixed => "As mixed",
            Self::Streaming => "Streaming and social (−14 LUFS)",
            Self::Podcast => "Podcast (−16 LUFS)",
        }
    }

    /// The target the renderer takes, through the CLI's own type.
    fn target(self) -> Option<LoudnessTarget> {
        self.lufs().and_then(|lufs| LoudnessTarget::new(lufs).ok())
    }
}

/// The render settings as the dialog holds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Choices {
    /// An index into [`SIZES`]: the same choice whatever the shape.
    size: usize,
    /// `None` is the edit's own rate — the one that conforms nothing.
    rate: Option<Fps>,
    /// The kind of file. Its codecs are the container's defaults.
    container: Container,
    /// Where the soundtrack is headed; the mix as balanced unless said.
    loudness: Loudness,
}

impl Default for Choices {
    fn default() -> Self {
        Self {
            size: 0,
            rate: None,
            container: Container::DEFAULT,
            loudness: Loudness::Mixed,
        }
    }
}

impl Choices {
    /// The renderer's settings for a project shaped `aspect` whose own rate is
    /// `own`.
    pub(crate) fn settings(&self, aspect: Ratio, own: Fps) -> RenderSettings {
        let (_, lines) = SIZES.get(self.size).copied().unwrap_or(SIZES[0]);
        RenderSettings::new(sized(aspect, lines), self.rate.unwrap_or(own))
            .with_format(OutputFormat::defaults_for(self.container))
            .with_loudness(self.loudness.target())
    }

    /// The container chosen, which also names the file's extension.
    pub(crate) fn container(&self) -> Container {
        self.container
    }

    /// The menus. The sizes are in the project's shape, `aspect`; `own`
    /// is the edit's rate, named in its option.
    pub(crate) fn show(&mut self, ui: &mut Ui, aspect: Ratio, own: Fps) {
        egui::Grid::new("render choices")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("File");
                ComboBox::from_id_salt("render file")
                    .selected_text(kind(self.container))
                    .show_ui(ui, |ui| {
                        for container in Container::ALL {
                            ui.selectable_value(&mut self.container, container, kind(container));
                        }
                    });
                ui.end_row();
                ui.label("Loudness");
                ComboBox::from_id_salt("render loudness")
                    .selected_text(self.loudness.words())
                    .show_ui(ui, |ui| {
                        for each in Loudness::ALL {
                            ui.selectable_value(&mut self.loudness, each, each.words());
                        }
                    });
                ui.end_row();
                // A sound-only file has no picture for a size or a rate to
                // describe, so neither is offered rather than both ignored.
                if self.container.is_sound_only() {
                    return;
                }
                ui.label("Size");
                ComboBox::from_id_salt("render size")
                    .selected_text(
                        SIZES
                            .get(self.size)
                            .map_or_else(String::new, |size| label(*size, aspect)),
                    )
                    .show_ui(ui, |ui| {
                        for (index, size) in SIZES.iter().enumerate() {
                            ui.selectable_value(&mut self.size, index, label(*size, aspect));
                        }
                    });
                ui.end_row();
                ui.label("Frame rate");
                ComboBox::from_id_salt("render rate")
                    .selected_text(rate(self.rate, own))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.rate, None, rate(None, own));
                        for fps in RATES {
                            ui.selectable_value(&mut self.rate, Some(fps), rate(Some(fps), own));
                        }
                    });
                ui.end_row();
            });
    }
}

/// The raster `lines` on the shorter side of a frame shaped `aspect`: its
/// delivery size scaled, each side rounded to even as every framed raster is.
fn sized(aspect: Ratio, lines: u32) -> Resolution {
    let (width, height) = aspect.delivery();
    let scale = f64::from(lines) / f64::from(DELIVERY_LINES);
    let side = |pixels: u32| even(f64::from(pixels) * scale);
    // `even` never goes below two, so neither side is ever zero.
    Resolution::new(side(width), side(height)).unwrap_or(Resolution::HD)
}

/// A size as the menu says it: `1080p — 1080 × 1920`.
fn label((name, lines): (&str, u32), aspect: Ratio) -> String {
    let raster = sized(aspect, lines);
    format!("{name} — {} × {}", raster.width(), raster.height())
}

/// A container in words a person reads: what it is, then its extension.
pub(crate) fn kind(container: Container) -> String {
    let what = if container.is_sound_only() {
        "Sound"
    } else {
        "Video"
    };
    format!("{what} — .{}", container.name())
}

/// A rate as the menu says it.
fn rate(chosen: Option<Fps>, own: Fps) -> String {
    match chosen {
        None => format!("Same as the edit ({own} fps)"),
        Some(fps) => format!("{fps} fps"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pressing Render without touching anything: 1080p, the edit's own rate,
    /// an mp4 of H.264 and AAC — what `scorsese render` makes by default too.
    #[test]
    fn the_defaults_are_the_file_most_people_want() {
        let settings = Choices::default().settings(Ratio::WIDE, Fps::PAL);
        assert_eq!(settings.resolution, Resolution::HD);
        let upright = Choices::default().settings(Ratio::TALL, Fps::PAL);
        assert_eq!(upright.resolution.to_string(), "1080x1920");
        assert_eq!(settings.fps, Fps::PAL);
        assert_eq!(settings.format, OutputFormat::default());
        assert_eq!(settings.loudness, None);
    }

    /// The two destinations reach the renderer as the targets the web sends.
    #[test]
    fn a_chosen_loudness_reaches_the_settings() {
        let aim = |loudness| {
            let chosen = Choices {
                loudness,
                ..Choices::default()
            };
            chosen.settings(Ratio::WIDE, Fps::THIRTY).loudness
        };
        assert_eq!(aim(Loudness::Mixed), None);
        assert_eq!(
            aim(Loudness::Streaming).map(LoudnessTarget::lufs),
            Some(-14.0)
        );
        assert_eq!(
            aim(Loudness::Podcast).map(LoudnessTarget::lufs),
            Some(-16.0)
        );
    }

    /// Every size on offer is in the project's shape, and says what it is.
    #[test]
    fn every_size_on_offer_is_in_the_projects_shape() {
        let offered = |aspect: Ratio| -> Vec<String> {
            SIZES.iter().map(|size| label(*size, aspect)).collect()
        };
        assert_eq!(
            offered(Ratio::WIDE),
            [
                "1080p — 1920 × 1080",
                "720p — 1280 × 720",
                "4K — 3840 × 2160"
            ]
        );
        assert_eq!(
            offered(Ratio::TALL),
            [
                "1080p — 1080 × 1920",
                "720p — 720 × 1280",
                "4K — 2160 × 3840"
            ]
        );
        let post: Ratio = "4:5".parse().expect("a shape");
        assert_eq!(
            offered(post),
            [
                "1080p — 1080 × 1350",
                "720p — 720 × 900",
                "4K — 2160 × 2700"
            ]
        );
        let chosen = Choices {
            size: 1,
            ..Choices::default()
        };
        let resolution = chosen.settings(post, Fps::THIRTY).resolution;
        assert_eq!(resolution.to_string(), "720x900");
    }

    #[test]
    fn a_chosen_rate_and_file_reach_the_settings() {
        let chosen = Choices {
            rate: Some(Fps::SIXTY),
            container: Container::Mp3,
            ..Choices::default()
        };
        let settings = chosen.settings(Ratio::WIDE, Fps::THIRTY);
        assert_eq!(settings.fps, Fps::SIXTY);
        assert!(!settings.format.has_picture());
        assert_eq!(kind(Container::Mp3), "Sound — .mp3");
        assert_eq!(kind(Container::Mkv), "Video — .mkv");
    }
}
