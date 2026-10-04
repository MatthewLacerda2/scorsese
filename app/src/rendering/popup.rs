//! The window a render shows while it runs: a bar, a percentage, what stage it
//! is at, and Stop — Filmora's export window, in spirit.
//!
//! **The stage is said beside the bar, always.** The percentage counts frames
//! only ([`scorsese_render::Reading::percent`] has why): it sits at 0 while the
//! media is measured and the sound is mixed, and at 99 while the encoder
//! finishes the file. A bar alone would look stuck at both ends, which is
//! exactly when somebody is watching it; the sentence beside it says it is not.

use std::time::Duration;

use egui::{Align2, Context, ProgressBar, RichText, Ui, Window};
use scorsese_render::{Phase, Reading};

use super::job::{Ended, Job};

/// How often the bar is redrawn while nothing else asks for a repaint: often
/// enough to look like it is moving, rarely enough to cost nothing.
const REDRAW: Duration = Duration::from_millis(100);

/// Draws the popup. True once it has been dismissed — after the render has
/// answered and Close was pressed.
pub(super) fn show(ctx: &Context, job: &mut Job) -> bool {
    let ended = job.ended().cloned();
    if job.moving() {
        ctx.request_repaint_after(REDRAW);
    }
    let mut dismissed = false;
    Window::new("Rendering")
        .collapsible(false)
        .resizable(false)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            ui.set_width(380.0);
            destination(ui, job);
            ui.add_space(6.0);
            gauge(ui, job.reading());
            ui.add_space(6.0);
            dismissed = outcome(ui, job, ended.as_ref());
        });
    dismissed
}

/// The file being made: its name, and the folder it goes in.
fn destination(ui: &mut Ui, job: &Job) {
    let out = job.out();
    let name = out.file_name().map_or_else(
        || out.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    ui.label(RichText::new(name).strong());
    if let Some(folder) = out.parent() {
        ui.label(RichText::new(folder.display().to_string()).weak().small());
    }
}

/// The bar, the percentage on it, and the stage and frame count under it.
fn gauge(ui: &mut Ui, reading: Reading) {
    let percent = reading.percent();
    ui.add(
        ProgressBar::new(f32::from(percent) / 100.0)
            .text(format!("{percent}%"))
            .desired_width(ui.available_width()),
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new(stage(reading.phase)).small());
        if reading.of > 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let count = format!("{} of {} frames", reading.done, reading.of);
                ui.label(RichText::new(count).weak().small());
            });
        }
    });
}

/// Stop while it runs; how it came out, and Close, once it has.
fn outcome(ui: &mut Ui, job: &Job, ended: Option<&Ended>) -> bool {
    let said = match ended {
        None => {
            let stopping = job.stopping();
            let label = if stopping { "Stopping…" } else { "Stop" };
            if ui
                .add_enabled(!stopping, egui::Button::new(label))
                .on_hover_text("Stops the render and deletes the unfinished file")
                .clicked()
            {
                job.stop();
            }
            return false;
        }
        Some(Ended::Wrote(what)) => RichText::new(format!("Saved — {what}")),
        Some(Ended::Stopped) => RichText::new("Stopped. Nothing was saved.").weak(),
        Some(Ended::Failed(why)) => {
            RichText::new(format!("Not rendered — {why}")).color(ui.visuals().warn_fg_color)
        }
    };
    ui.label(said);
    ui.add_space(4.0);
    ui.button("Close").clicked()
}

/// What the render is doing, in a person's words.
pub(super) fn stage(phase: Phase) -> &'static str {
    match phase {
        Phase::Waiting | Phase::Preparing => "Getting the media ready…",
        Phase::Mixing => "Mixing the sound…",
        Phase::Drawing => "Drawing the frames…",
        Phase::Finishing => "Finishing the file…",
        Phase::Done => "Done",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every stage the bar sits still through has a sentence of its own, so a
    /// reading at 0% or 99% is never only a number.
    #[test]
    fn the_still_stretches_of_the_bar_each_say_what_is_happening() {
        let still = [Phase::Preparing, Phase::Mixing, Phase::Finishing];
        let words: Vec<_> = still.iter().map(|phase| stage(*phase)).collect();
        assert!(words.iter().all(|word| word.ends_with('…')));
        assert_ne!(words[0], words[1]);
        assert_ne!(words[1], words[2]);
        assert_eq!(stage(Phase::Done), "Done");
    }
}
