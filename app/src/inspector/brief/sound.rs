//! The brief behind a generated sound effect: a description, a length, and
//! whether it loops.
//!
//! The fifth brief, [`track`](super::track)'s sibling and kept apart from it
//! for [`super`]'s reason: a sound is `generated_audio` too, but its length is
//! in fractions of a second and capped at thirty, and it is billed by its own
//! rate. One model is offered, so there is nothing to pick.
//!
//! **The length is the price**, as it is for a track: with no length the cost
//! line says so instead of a figure, and the field starts from the clip it
//! sits under without writing anything until it is touched.

use egui::{Grid, RichText, TextEdit, Ui};
use scorsese_core::{
    Asset, AssetId, AudioBlock, Fps, Frames, MAX_SOUND_SECONDS, MIN_SOUND_SECONDS, Project,
    SoundRequest,
};
use scorsese_providers::prices::{dollars, from_cents};
use scorsese_providers::sound::quote;

use crate::inspector::Inspector;
use crate::inspector::number;
use crate::inspector::selected::Selected;
use crate::project::Open;

/// A generated sound's brief, read out of the document in one pass.
pub(in crate::inspector) struct Sound {
    /// The asset the brief belongs to.
    asset: AssetId,
    /// What the sound is.
    prompt: String,
    /// Length and loop.
    request: SoundRequest,
}

impl Sound {
    /// Reads the brief of `asset`, or `None` when it is not a generated sound.
    pub(super) fn of(project: &Project, asset: &AssetId) -> Option<Self> {
        let found = project.asset(asset)?;
        if found.audio_block() != Some(AudioBlock::Sound) {
            return None;
        }
        Some(Self {
            asset: asset.clone(),
            prompt: found.prompt.clone().unwrap_or_default(),
            request: found.sound.clone().unwrap_or_default(),
        })
    }
}

impl Inspector {
    /// Draws the sound's fields, under the heading [`Inspector::brief`] wrote.
    pub(super) fn sound(
        &mut self,
        ui: &mut Ui,
        open: &mut Open,
        selected: &Selected,
        brief: &Sound,
    ) {
        ui.label("Prompt");
        let mut prompt = brief.prompt.clone();
        let edited = ui
            .add(
                TextEdit::multiline(&mut prompt)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("what the sound is"),
            )
            .changed();
        if edited {
            self.attempt_brief(open, selected, &brief.asset, "the prompt", move |asset| {
                asset.prompt = Some(prompt);
            });
        }
        ui.label(RichText::new(cost(brief.request.seconds)).weak().small());

        let fps = open.project.timeline_fps;
        Grid::new("sound-brief")
            .num_columns(3)
            .min_col_width(number::LABELS)
            .show(ui, |ui| {
                ui.label("Length");
                let set = brief.request.seconds;
                let mut seconds = set.unwrap_or_else(|| covering(selected.duration, fps));
                let changed = number::field(ui, &mut seconds, "", "s", |drag| {
                    drag.speed(0.1)
                        .max_decimals(1)
                        .range(MIN_SOUND_SECONDS..=MAX_SOUND_SECONDS)
                })
                .on_hover_text("How long the sound is made — drag, or click to type")
                .changed();
                ui.label(
                    RichText::new(if set.is_some() {
                        format!("{MIN_SOUND_SECONDS}–{MAX_SOUND_SECONDS}s")
                    } else {
                        String::from("not set yet — the clip's length is suggested")
                    })
                    .weak()
                    .small(),
                );
                ui.end_row();
                if changed {
                    self.attempt_brief(open, selected, &brief.asset, "the length", move |asset| {
                        request_of(asset).seconds = Some(seconds);
                    });
                }

                ui.label("Loop");
                let mut looping = brief.request.looping;
                let toggled = ui.checkbox(&mut looping, "").changed();
                ui.label(RichText::new("repeats seamlessly").weak().small());
                ui.end_row();
                if toggled {
                    self.attempt_brief(open, selected, &brief.asset, "loop", move |asset| {
                        request_of(asset).looping = looping;
                    });
                }
            });
    }
}

/// What the sound will cost, or why it cannot be said yet.
///
/// The figure is [`quote`]'s — the one the ceiling is checked against — so it
/// can never disagree with what somebody is charged.
fn cost(seconds: Option<f64>) -> String {
    match seconds.filter(|seconds| SoundRequest::takes(*seconds)) {
        Some(seconds) => format!(
            "{seconds}s · about {}, by our own calculation",
            dollars(from_cents(quote(millis(seconds))), 2)
        ),
        None => String::from("no length yet — a sound is priced by how long it runs"),
    }
}

/// `seconds` in whole milliseconds, as the provider's brief settles them.
/// Only called on a length [`SoundRequest::takes`], so the cast is in range.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn millis(seconds: f64) -> u32 {
    (seconds * 1000.0).round() as u32
}

/// The tenths of a second that cover `duration`, within what the vendor makes.
fn covering(duration: Frames, fps: Fps) -> f64 {
    let tenths = (fps.seconds(duration) * 10.0).ceil() / 10.0;
    tenths.clamp(MIN_SOUND_SECONDS, MAX_SOUND_SECONDS)
}

/// The asset's request, put there if it had none — absent means every default,
/// so the first edit materialises the block rather than being dropped.
fn request_of(asset: &mut Asset) -> &mut SoundRequest {
    asset.sound.get_or_insert_with(SoundRequest::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{AssetKind, MusicRequest};

    /// A project holding one sketched sound of `seconds`, and one track.
    fn sounded(seconds: Option<f64>) -> Project {
        let mut project = Project::new("test", Fps::THIRTY);
        let mut sound = Asset::sketch(AssetId::new("door"), AssetKind::GeneratedAudio, "a door");
        sound.sound = Some(SoundRequest {
            seconds,
            ..SoundRequest::default()
        });
        let mut track = Asset::sketch(AssetId::new("score"), AssetKind::GeneratedAudio, "piano");
        track.music = Some(MusicRequest::default());
        project.assets = vec![sound, track];
        project
    }

    /// A sound has this brief and a track does not: the two are told apart by
    /// the block, the way every pass tells them apart.
    #[test]
    fn a_sound_has_a_sound_brief_and_a_track_does_not() {
        let project = sounded(Some(2.5));
        let sound = Sound::of(&project, &AssetId::new("door")).expect("a sound");
        assert_eq!(sound.prompt, "a door");
        assert_eq!(sound.request.seconds, Some(2.5));
        assert!(Sound::of(&project, &AssetId::new("score")).is_none());
        // Nor is a sound a line: it has no voice to pick.
        assert!(super::super::line::Line::of(&project, &AssetId::new("door")).is_none());
    }

    /// The cost is the provider's quote, and with no length there is none.
    #[test]
    fn the_cost_is_the_quote_or_says_why_there_is_none() {
        let said = cost(Some(2.5));
        assert!(
            said.contains(&dollars(from_cents(quote(2_500)), 2)),
            "{said}"
        );
        assert!(cost(None).starts_with("no length yet"));
        assert!(cost(Some(90.0)).starts_with("no length yet"));
    }

    /// The suggested length covers the clip to the tenth, and stays inside
    /// what the vendor makes at either end.
    #[test]
    fn the_suggested_length_covers_the_clip_within_the_vendor_range() {
        assert!((covering(Frames(61), Fps::THIRTY) - 2.1).abs() < 1e-9);
        assert_eq!(covering(Frames(1), Fps::THIRTY), MIN_SOUND_SECONDS);
        assert_eq!(covering(Frames(u64::MAX), Fps::THIRTY), MAX_SOUND_SECONDS);
    }

    /// The first edit to a sound with no block writes one.
    #[test]
    fn the_first_edit_materialises_the_block_with_its_defaults() {
        let mut asset = Asset::sketch(AssetId::new("door"), AssetKind::GeneratedAudio, "a door");
        request_of(&mut asset).looping = true;
        assert_eq!(
            asset.sound,
            Some(SoundRequest {
                seconds: None,
                looping: true,
            })
        );
    }
}
