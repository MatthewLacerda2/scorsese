//! What a render is asked to make: the plain settings, and their defaults.
//!
//! Three choices and no more — size, frame rate, kind of file — each with a
//! default good enough that pressing Render without touching any of them gives
//! the file most people want: a 1080p mp4 at the edit's own rate. Bitrates,
//! codecs and sample rates stay at the renderer's defaults; they are how the
//! file is made, not what it is, and the CLI is where somebody who cares about
//! them already is.

use egui::{ComboBox, Ui};
use scorsese_core::Fps;
use scorsese_render::{Container, OutputFormat, RenderSettings, Resolution};

/// The sizes on offer, as `(label, width, height)`. The shape of the frame is
/// the size's, since a project has no canvas of its own to ask.
const SIZES: [(&str, u32, u32); 5] = [
    ("1080p — 1920 × 1080", 1920, 1080),
    ("720p — 1280 × 720", 1280, 720),
    ("4K — 3840 × 2160", 3840, 2160),
    ("Vertical — 1080 × 1920", 1080, 1920),
    ("Square — 1080 × 1080", 1080, 1080),
];

/// The rates on offer besides the edit's own.
const RATES: [Fps; 4] = [Fps::FILM, Fps::PAL, Fps::THIRTY, Fps::SIXTY];

/// The render settings as the dialog holds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Choices {
    /// An index into [`SIZES`].
    size: usize,
    /// `None` is the edit's own rate — the one that conforms nothing.
    rate: Option<Fps>,
    /// The kind of file. Its codecs are the container's defaults.
    container: Container,
}

impl Default for Choices {
    fn default() -> Self {
        Self {
            size: 0,
            rate: None,
            container: Container::DEFAULT,
        }
    }
}

impl Choices {
    /// The renderer's settings for a project whose own rate is `own`.
    pub(crate) fn settings(&self, own: Fps) -> RenderSettings {
        let (_, width, height) = SIZES.get(self.size).copied().unwrap_or(SIZES[0]);
        let resolution = Resolution::new(width, height).unwrap_or(Resolution::HD);
        RenderSettings::new(resolution, self.rate.unwrap_or(own))
            .with_format(OutputFormat::defaults_for(self.container))
    }

    /// The container chosen, which also names the file's extension.
    pub(crate) fn container(&self) -> Container {
        self.container
    }

    /// The three menus. `own` is the edit's rate, named in its option.
    pub(crate) fn show(&mut self, ui: &mut Ui, own: Fps) {
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
                // A sound-only file has no picture for a size or a rate to
                // describe, so neither is offered rather than both ignored.
                if self.container.is_sound_only() {
                    return;
                }
                ui.label("Size");
                ComboBox::from_id_salt("render size")
                    .selected_text(SIZES.get(self.size).map_or("", |size| size.0))
                    .show_ui(ui, |ui| {
                        for (index, (label, ..)) in SIZES.iter().enumerate() {
                            ui.selectable_value(&mut self.size, index, *label);
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
        let settings = Choices::default().settings(Fps::PAL);
        assert_eq!(settings.resolution, Resolution::HD);
        assert_eq!(settings.fps, Fps::PAL);
        assert_eq!(settings.format, OutputFormat::default());
    }

    /// A size the renderer refused would fall back to 1080p without a word, so
    /// every one on offer is held to being legal.
    #[test]
    fn every_size_on_offer_is_one_a_render_accepts() {
        for (index, (label, width, height)) in SIZES.iter().enumerate() {
            let chosen = Choices {
                size: index,
                ..Choices::default()
            };
            let resolution = chosen.settings(Fps::THIRTY).resolution;
            assert_eq!(
                (resolution.width(), resolution.height()),
                (*width, *height),
                "{label}"
            );
        }
    }

    #[test]
    fn a_chosen_rate_and_file_reach_the_settings() {
        let chosen = Choices {
            rate: Some(Fps::SIXTY),
            container: Container::Mp3,
            ..Choices::default()
        };
        let settings = chosen.settings(Fps::THIRTY);
        assert_eq!(settings.fps, Fps::SIXTY);
        assert!(!settings.format.has_picture());
        assert_eq!(kind(Container::Mp3), "Sound — .mp3");
        assert_eq!(kind(Container::Mkv), "Video — .mkv");
    }
}
