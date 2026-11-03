//! The brief behind a generated track of music: a description, a length, and
//! whether anyone sings.
//!
//! The fourth brief, beside [`line`](super::line), and kept apart from it for
//! [`super`]'s reason: a track is `generated_audio` too, but it has no voice,
//! no model and no language, and it is billed by the minute rather than by the
//! character. One model is offered, so there is nothing to pick there either.
//!
//! **The length is the price.** A track with no length cannot be quoted, so
//! the cost line says that instead of a figure, and the field starts from the
//! clip it sits under — the length a person would most often give it — without
//! writing anything until it is touched.

use egui::{Grid, RichText, TextEdit, Ui};
use scorsese_core::{
    Asset, AssetId, AudioBlock, Fps, Frames, MAX_MUSIC_SECONDS, MIN_MUSIC_SECONDS, MusicRequest,
    Project,
};
use scorsese_providers::music::quote;
use scorsese_providers::prices::{dollars, from_cents};

use crate::inspector::Inspector;
use crate::inspector::number;
use crate::inspector::selected::Selected;
use crate::project::Open;

/// A generated track's brief, read out of the document in one pass.
pub(in crate::inspector) struct Track {
    /// The asset the brief belongs to.
    asset: AssetId,
    /// What the music sounds like.
    prompt: String,
    /// Length and instrumental.
    request: MusicRequest,
}

impl Track {
    /// Reads the brief of `asset`, or `None` when it is not a generated track.
    pub(super) fn of(project: &Project, asset: &AssetId) -> Option<Self> {
        let found = project.asset(asset)?;
        if found.audio_block() != Some(AudioBlock::Music) {
            return None;
        }
        Some(Self {
            asset: asset.clone(),
            prompt: found.prompt.clone().unwrap_or_default(),
            request: found.music.clone().unwrap_or_default(),
        })
    }
}

impl Inspector {
    /// Draws the track's fields, under the heading [`Inspector::brief`] wrote.
    pub(super) fn track(
        &mut self,
        ui: &mut Ui,
        open: &mut Open,
        selected: &Selected,
        brief: &Track,
    ) {
        ui.label("Prompt");
        let mut prompt = brief.prompt.clone();
        let edited = ui
            .add(
                TextEdit::multiline(&mut prompt)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("what the music sounds like"),
            )
            .changed();
        if edited {
            self.attempt_brief(open, selected, &brief.asset, "the prompt", move |asset| {
                asset.prompt = Some(prompt);
            });
        }
        ui.label(RichText::new(cost(brief.request.seconds)).weak().small());

        let fps = open.project.timeline_fps;
        Grid::new("track-brief")
            .num_columns(3)
            .min_col_width(number::LABELS)
            .show(ui, |ui| {
                ui.label("Length");
                let set = brief.request.seconds;
                let mut seconds = set.unwrap_or_else(|| covering(selected.duration, fps));
                let changed = number::field(ui, &mut seconds, "", "s", |drag| {
                    drag.range(MIN_MUSIC_SECONDS..=MAX_MUSIC_SECONDS)
                })
                .on_hover_text("How long the track is composed — drag, or click to type")
                .changed();
                ui.label(
                    RichText::new(if set.is_some() {
                        format!("{MIN_MUSIC_SECONDS}–{MAX_MUSIC_SECONDS}s")
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

                ui.label("Instrumental");
                let mut instrumental = brief.request.instrumental;
                let toggled = ui.checkbox(&mut instrumental, "").changed();
                ui.label(RichText::new("no singing").weak().small());
                ui.end_row();
                if toggled {
                    self.attempt_brief(
                        open,
                        selected,
                        &brief.asset,
                        "instrumental",
                        move |asset| {
                            request_of(asset).instrumental = instrumental;
                        },
                    );
                }
            });
    }
}

/// What the track will cost, or why it cannot be said yet.
///
/// The figure is [`quote`]'s — the one the ceiling is checked against — so
/// it can never disagree with what somebody is charged.
fn cost(seconds: Option<u32>) -> String {
    match seconds {
        Some(seconds) => format!(
            "{seconds}s · about {}, by our own calculation",
            dollars(from_cents(quote(seconds)), 2)
        ),
        None => String::from("no length yet — a track is priced by how long it runs"),
    }
}

/// The whole seconds that cover `duration`, within what the vendor composes.
fn covering(duration: Frames, fps: Fps) -> u32 {
    let seconds = fps.seconds(duration).ceil();
    // Clamped as a float first, so a clip of hours cannot overflow the cast.
    seconds.clamp(f64::from(MIN_MUSIC_SECONDS), f64::from(MAX_MUSIC_SECONDS)) as u32
}

/// The asset's request, put there if it had none — absent means every default,
/// so the first edit materialises the block rather than being dropped.
fn request_of(asset: &mut Asset) -> &mut MusicRequest {
    asset.music.get_or_insert_with(MusicRequest::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::AssetKind;

    /// A project holding one sketched track of `seconds`, and one line.
    fn scored(seconds: Option<u32>) -> Project {
        let mut project = Project::new("test", Fps::THIRTY);
        let mut track = Asset::sketch(
            AssetId::new("score"),
            AssetKind::GeneratedAudio,
            "warm piano",
        );
        track.music = Some(MusicRequest {
            seconds,
            ..MusicRequest::default()
        });
        let line = Asset::sketch(AssetId::new("vo"), AssetKind::GeneratedAudio, "Hello.");
        project.assets = vec![track, line];
        project
    }

    /// A track has this brief and a line does not: the two are told apart by
    /// the block, the way every pass tells them apart.
    #[test]
    fn a_track_has_a_track_brief_and_a_line_does_not() {
        let project = scored(Some(30));
        let track = Track::of(&project, &AssetId::new("score")).expect("a track");
        assert_eq!(track.prompt, "warm piano");
        assert_eq!(track.request.seconds, Some(30));
        assert!(Track::of(&project, &AssetId::new("vo")).is_none());
    }

    /// The cost is the provider's quote, and with no length there is none.
    #[test]
    fn the_cost_is_the_quote_or_says_why_there_is_none() {
        let said = cost(Some(60));
        assert!(said.contains(&dollars(from_cents(quote(60)), 2)), "{said}");
        assert!(cost(None).starts_with("no length yet"));
    }

    /// The suggested length covers the clip, and stays inside what the vendor
    /// composes at either end.
    #[test]
    fn the_suggested_length_covers_the_clip_within_the_vendor_range() {
        assert_eq!(covering(Frames(301), Fps::THIRTY), 11);
        assert_eq!(covering(Frames(1), Fps::THIRTY), MIN_MUSIC_SECONDS);
        assert_eq!(covering(Frames(u64::MAX), Fps::THIRTY), MAX_MUSIC_SECONDS);
    }

    /// The first edit to a track with no block writes one, keeping the
    /// instrumental default rather than dropping it.
    #[test]
    fn the_first_edit_materialises_the_block_with_its_defaults() {
        let mut asset = Asset::sketch(AssetId::new("score"), AssetKind::GeneratedAudio, "piano");
        request_of(&mut asset).seconds = Some(12);
        assert_eq!(
            asset.music,
            Some(MusicRequest {
                seconds: Some(12),
                instrumental: true,
            })
        );
    }
}
