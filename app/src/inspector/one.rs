//! The panel with exactly one clip selected: what it is, the plain values a
//! person changes with a mouse, and what animates it.
//!
//! Its own module because it is the *subject* case and there is now more than
//! one. Everything here needs a single clip to be about — a start field, a
//! duration field, a fit — and the moment the selection holds three of them
//! none of these questions has an answer. [`super::several`] is what the panel
//! shows instead, and keeping the two apart is what stops either from growing
//! a branch on how many are selected.

use egui::{Grid, RichText, Ui};
use scorsese_core::{AssetKind, GenerationState};

use super::Inspector;
use super::controls::{fit_row, frames_row, speed_row};
use super::selected::Selected;
use crate::project::Open;

impl Inspector {
    /// Draws the panel for one clip.
    pub(super) fn one(&mut self, ui: &mut Ui, open: &mut Open, selected: &Selected) {
        identity(ui, selected);
        ui.add_space(8.0);
        self.controls(ui, open, selected);
        // Under the clip's own fields, because it is about the *asset*: a
        // person reads what this clip is, then what the shot behind it asked
        // for. Absent for everything that is not a generated shot, which is
        // most things.
        if let Some(brief) = super::Brief::of(&open.project, &selected.asset) {
            self.brief(ui, open, selected, &brief);
        }
        self.animation(ui, open, selected);
    }

    /// The plain controls: where the clip starts, how long it runs, and how it
    /// meets the raster. One value and one field each — anything with structure
    /// to it does not belong in a panel.
    ///
    /// A change is committed on the frame it happens rather than when the drag
    /// ends, so the timeline moves with the number and the file is never a
    /// gesture behind the window. A value the project refuses simply never
    /// lands, and the field springs back to what the document still says —
    /// which is what dragging a clip into its neighbour feels like: a wall.
    fn controls(&mut self, ui: &mut Ui, open: &mut Open, selected: &Selected) {
        let fps = open.project.timeline_fps;
        Grid::new("clip").num_columns(3).show(ui, |ui| {
            if let Some(start) = frames_row(ui, "Start", selected.start, fps) {
                self.attempt(open, selected, "start", move |clip| clip.start = start);
            }
            if let Some(duration) = frames_row(ui, "Duration", selected.duration, fps) {
                self.attempt(open, selected, "duration", move |clip| {
                    clip.duration = duration;
                });
            }
            // Retimes rather than writing the rate on its own: this is a
            // person picking 2× on a menu, and what that means to them is the
            // same footage in half the time. The document keeps the rate and
            // the length apart, and this is the panel spending that freedom.
            if let Some(speed) = speed_row(ui, selected.speed, selected.duration, fps) {
                self.attempt(open, selected, "speed", move |clip| clip.retime(speed));
            }
            // Picture only. A sound has no raster, so a fit control on an audio
            // clip would be a choice with nothing on the other end of it.
            if selected.picture
                && let Some(fit) = fit_row(ui, selected.fit)
            {
                self.attempt(open, selected, "fit", move |clip| clip.fit = fit);
            }
        });
    }

    /// What animates this clip.
    ///
    /// Said, never offered as a value. The one operation here is deleting a
    /// track a tool signed — **whole**, which is the only way the format allows
    /// anyone to touch one. A hand-written ramp gets no button at all: it is
    /// work somebody did, this window has no undo, and re-running a tool is not
    /// a way to get it back.
    fn animation(&mut self, ui: &mut Ui, open: &mut Open, selected: &Selected) {
        if selected.animated.is_empty() {
            return;
        }
        ui.add_space(10.0);
        ui.label(RichText::new("ANIMATED").strong().small());
        for animated in &selected.animated {
            let property = animated.property.as_str();
            ui.horizontal(|ui| {
                ui.label(property).on_hover_text(match &animated.by {
                    Some(by) => format!("{by} wrote this ramp, and may rewrite it"),
                    None => "set by hand — nothing here will flatten it into one value".to_owned(),
                });
                ui.label(RichText::new(points(animated.points)).weak().small());
                if let Some(by) = animated.by.clone() {
                    ui.label(RichText::new(format!("· {by}")).weak().small());
                    if ui
                        .small_button("✕")
                        .on_hover_text(format!("Delete the whole ramp {by} wrote for {property}"))
                        .clicked()
                    {
                        let path = animated.property.clone();
                        self.attempt(open, selected, format!("the {by} ramp"), move |clip| {
                            // That one track and no other: the row is about one
                            // property, and withdrawing everything a tool wrote
                            // is a different and much larger operation.
                            clip.keyframes.retain(|track| {
                                track.property != path || track.by.as_deref() != Some(by.as_str())
                            });
                        });
                    }
                }
            });
        }
    }
}

/// What this clip is: the asset it shows, what that asset is, and where it sits.
fn identity(ui: &mut Ui, selected: &Selected) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(selected.asset.as_str()).strong());
        ui.label(RichText::new(describe(selected)).weak().small());
    });
    ui.label(
        RichText::new(format!("on track {}", selected.track))
            .weak()
            .small(),
    );
}

/// The asset's kind and, for one that does not exist yet, how far along it is.
///
/// The lifecycle belongs here because it answers the question a slug card in
/// the preview raises: this clip is a brief, and nothing has made it yet.
fn describe(selected: &Selected) -> String {
    let kind = selected.kind.map_or("not in the assets table", kind_name);
    match selected.state {
        Some(state) => format!("{kind} · {}", state_name(state)),
        None => kind.to_owned(),
    }
}

/// What a kind is called on screen.
///
/// Spelled out rather than derived from the serde name, because `synth_audio`
/// is a field value and "synthesised audio" is what it is. Exhaustive on
/// purpose: a kind added to the format fails to compile here, which is the
/// cheapest possible reminder that the window has to name it too.
pub(super) fn kind_name(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Video => "video",
        AssetKind::Image => "image",
        AssetKind::Audio => "audio",
        AssetKind::Text => "text",
        AssetKind::Color => "colour",
        AssetKind::GeneratedVideo => "generated video",
        AssetKind::GeneratedAudio => "generated speech",
        AssetKind::SynthAudio => "synthesised audio",
    }
}

/// Where a generated asset is in the sketch lifecycle, in a word.
fn state_name(state: GenerationState) -> &'static str {
    match state {
        GenerationState::Sketch => "sketch, not made yet",
        GenerationState::Queued => "queued",
        GenerationState::Generated => "generated",
        GenerationState::Stale => "stale, the brief has changed",
    }
}

/// How many points a ramp has, said in English rather than as `1 points`.
fn points(count: usize) -> String {
    match count {
        1 => "1 point".to_owned(),
        many => format!("{many} points"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_point_is_not_1_points() {
        assert_eq!(points(1), "1 point");
        assert_eq!(points(4), "4 points");
    }

    /// A generated asset says where in the lifecycle it is, because that is the
    /// answer to why the preview is showing a card instead of a picture.
    #[test]
    fn a_kind_and_a_lifecycle_state_are_both_said() {
        assert_eq!(kind_name(AssetKind::SynthAudio), "synthesised audio");
        assert!(state_name(GenerationState::Sketch).starts_with("sketch"));
    }
}
