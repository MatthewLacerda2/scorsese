//! The status strip: what the edit *is*, in one line along the bottom.
//!
//! Everything on it is a number the window already computes and showed nowhere.
//! The length of the film is the sharpest example — [`crate::editing::length`]
//! runs every repaint, because the timeline has to fit to it and the playhead
//! has to stop at it, and *"how long is this so far"* is the question a person
//! cutting asks most often. It was reachable only by parking the playhead at the
//! end and reading the transport.
//!
//! Monospaced throughout, and that is the reason it is a strip rather than a
//! sentence: these are figures to be compared with the ones that were there a
//! minute ago, and figures compare by looking rather than by reading.

use egui::{Align, Layout, Ui};
use scorsese_core::Project;

use super::Scorsese;
use crate::editing::length;
use crate::theme::{marks, palette};
use crate::timeline::timecode;

/// Draws the strip for whatever is open, or the one thing there is to say when
/// nothing is.
pub(super) fn show(ui: &mut Ui, window: &Scorsese) {
    let Some(open) = window.project() else {
        ui.label(marks::figure_dim("no project"));
        return;
    };
    let project = &open.project;
    let fps = project.timeline_fps;

    ui.horizontal(|ui| {
        ui.label(marks::figure_dim(format!("{fps} fps")));
        separator(ui);
        ui.label(marks::figure(timecode(length(project), fps)))
            .on_hover_text("How long the edit runs — the end of the last clip on any track");
        separator(ui);
        ui.label(marks::figure_dim(counted(project)));

        // Right-aligned, because it is the half that changes as a hand moves:
        // a reading that shifts sideways every time the playhead does is one
        // the eye has to chase.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(marks::figure(format!(
                "{}  f {}",
                timecode(window.playhead(), fps),
                window.playhead().get()
            )));
            let selected = window.selected().len();
            if selected > 0 {
                separator(ui);
                ui.label(
                    marks::figure_dim(format!("{selected} selected")).color(palette::ACCENT_DIM),
                );
            }
        });
    });
}

/// The dot between two readings.
///
/// A character rather than [`Ui::separator`], which draws a full-height rule —
/// four of those in a strip this thin reads as a table with no rows in it.
fn separator(ui: &mut Ui) {
    ui.label(marks::figure_dim("·").color(palette::FAINT));
}

/// How much is in the edit: tracks, and the clips on them.
///
/// Singular and plural spelled out, because `1 clips` in a bar somebody reads a
/// hundred times an evening is the sort of thing that quietly makes a window
/// feel unfinished.
fn counted(project: &Project) -> String {
    let tracks = project.tracks.len();
    let clips = project.clips().count();
    format!(
        "{tracks} {}  {clips} {}",
        if tracks == 1 { "track" } else { "tracks" },
        if clips == 1 { "clip" } else { "clips" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{Fps, Track, TrackId, TrackKind};

    #[test]
    fn one_of_a_thing_is_not_said_in_the_plural() {
        let mut project = Project::new("t", Fps::THIRTY);
        assert_eq!(counted(&project), "0 tracks  0 clips");
        project
            .tracks
            .push(Track::new(TrackId::new("v1"), TrackKind::Video));
        assert_eq!(counted(&project), "1 track  0 clips");
    }
}
