//! Where a clip's picture sits in the frame: its position, rotation and scale,
//! as plain values.
//!
//! The format has no *field* for any of these. A layer is moved, turned and
//! sized by keyframe tracks — `transform.position.x` and the rest — and a clip
//! that simply sits somewhere else is one keyframe, held for its whole length.
//! That is what makes a field here honest rather than a keyframe editor in
//! disguise: a property with **no track, or one track of one point**, has
//! exactly one value, and changing it is `scorsese_core::level::set` with a
//! [`Level::Flat`] — the same call `set_volume` makes, so the window and an
//! assistant write the same document for the same request.
//!
//! Anything more than that is a ramp, and the inspector's rule stands: a ramp
//! is shown as animated and offered no single value, because one field cannot
//! hold one and typing a number over it would flatten work nobody asked to lose.

use egui::{DragValue, Grid, RichText, Ui};
use scorsese_core::{Clip, ClipId, KeyframeTrack, Level, LevelError, Project, PropertyPath, level};
use scorsese_render::picture::path::{POSITION_X, POSITION_Y, ROTATION, SCALE_X, SCALE_Y};

use super::Inspector;
use super::edit::{self, Refusal};
use super::selected::Selected;
use crate::project::Open;

/// Every property this panel offers as a value, with what it reads when nothing
/// sets it — which is the layer where it naturally sits, upright, at its own
/// size.
const FIELDS: [(&str, f64); 5] = [
    (POSITION_X, 0.0),
    (POSITION_Y, 0.0),
    (ROTATION, 0.0),
    (SCALE_X, 1.0),
    (SCALE_Y, 1.0),
];

/// What one of these properties reads across the whole clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Held {
    /// One value, from the first frame to the last. The only state a field
    /// can show and change without lying.
    Value(f64),
    /// Scale that is not the same both ways — something stretched this layer on
    /// purpose, so one "scale" number would be a claim about half of it.
    Stretched {
        /// Width multiplier.
        wide: f64,
        /// Height multiplier.
        tall: f64,
    },
    /// It moves over the clip. Said, never offered as a value.
    Animated,
}

/// A picture clip's placement in the frame, as far as the panel shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Transform {
    /// Offset right, as a fraction of the frame's width.
    pub(super) x: Held,
    /// Offset down, as a fraction of the frame's height.
    pub(super) y: Held,
    /// Degrees clockwise.
    pub(super) rotation: Held,
    /// Size multiplier, the same both ways.
    pub(super) scale: Held,
}

impl Transform {
    /// Reads the placement out of a clip.
    pub(super) fn of(clip: &Clip) -> Self {
        let scale = match (held(clip, SCALE_X, 1.0), held(clip, SCALE_Y, 1.0)) {
            (Held::Value(wide), Held::Value(tall)) if wide == tall => Held::Value(wide),
            (Held::Value(wide), Held::Value(tall)) => Held::Stretched { wide, tall },
            _ => Held::Animated,
        };
        Self {
            x: held(clip, POSITION_X, 0.0),
            y: held(clip, POSITION_Y, 0.0),
            rotation: held(clip, ROTATION, 0.0),
            scale,
        }
    }
}

/// True when `track` is one this panel already shows as a value, so the list
/// of what animates the clip does not say it a second time as "1 point".
pub(super) fn shown_as_value(clip: &Clip, track: &KeyframeTrack) -> bool {
    FIELDS
        .iter()
        .find(|(path, _)| track.property.as_str() == *path)
        .is_some_and(|(path, rest)| matches!(held(clip, path, *rest), Held::Value(_)))
}

/// What `path` reads across `clip`: its resting value when nothing sets it,
/// the one point when a single held keyframe does, and animated otherwise —
/// including two tracks on one property, which is not a value anybody can name.
fn held(clip: &Clip, path: &str, rest: f64) -> Held {
    let mut tracks = clip
        .keyframes
        .iter()
        .filter(|track| track.property.as_str() == path);
    match (tracks.next(), tracks.next()) {
        (None, _) => Held::Value(rest),
        (Some(track), None) => match track.keyframes.as_slice() {
            [only] => Held::Value(only.value),
            _ => Held::Animated,
        },
        (Some(_), Some(_)) => Held::Animated,
    }
}

impl Inspector {
    /// Draws position, rotation and scale for a picture clip.
    pub(super) fn transform(&mut self, ui: &mut Ui, open: &mut Open, selected: &Selected) {
        let Some(transform) = selected.transform else {
            return;
        };
        ui.add_space(6.0);
        Grid::new("transform").num_columns(2).show(ui, |ui| {
            ui.label("Position");
            ui.horizontal(|ui| {
                if let Some(x) = field(ui, transform.x, Unit::Percent, "x ") {
                    self.hold(open, selected, "position", &[POSITION_X], x);
                }
                if let Some(y) = field(ui, transform.y, Unit::Percent, "y ") {
                    self.hold(open, selected, "position", &[POSITION_Y], y);
                }
            })
            .response
            .on_hover_text(
                "How far from where it naturally sits: right and down, in % of the frame",
            );
            ui.end_row();

            ui.label("Rotation");
            if let Some(turn) = field(ui, transform.rotation, Unit::Degrees, "") {
                self.hold(open, selected, "rotation", &[ROTATION], turn);
            }
            ui.end_row();

            ui.label("Scale");
            if let Some(size) = field(ui, transform.scale, Unit::Size, "") {
                // Both axes, because this is the one scale a person means: the
                // picture bigger or smaller, in proportion.
                self.hold(open, selected, "scale", &[SCALE_X, SCALE_Y], size);
            }
            ui.end_row();
        });
    }

    /// Holds each of `paths` at `value` for the whole clip, or says why not.
    fn hold(
        &mut self,
        open: &mut Open,
        selected: &Selected,
        what: &str,
        paths: &[&str],
        value: f64,
    ) {
        let clip = &selected.clip;
        let outcome = edit::apply_to_project(open, |project| hold_all(project, clip, paths, value));
        self.refused = outcome.err().map(|problems| Refusal {
            clip: clip.clone(),
            what: what.to_owned(),
            problems,
        });
    }
}

/// Holds each of `paths` at `value` over one clip, through the same
/// `level::set` an assistant's `set_volume` goes through — each one replacing
/// whatever single point held it before.
fn hold_all(
    project: &mut Project,
    clip: &ClipId,
    paths: &[&str],
    value: f64,
) -> Result<(), Vec<String>> {
    for path in paths {
        level::set(project, clip, &PropertyPath::new(*path), Level::Flat(value))
            .map_err(problems)?;
    }
    Ok(())
}

/// Every problem a refused level carries, rather than the one-line summary.
fn problems(error: LevelError) -> Vec<String> {
    match error {
        LevelError::Refused(errors) => errors.into_vec().iter().map(ToString::to_string).collect(),
        other => vec![other.to_string()],
    }
}

/// How a value is shown in its field, and turned back into the document's.
#[derive(Debug, Clone, Copy)]
enum Unit {
    /// A fraction of the frame, shown as a percentage of it.
    Percent,
    /// Degrees, as they are.
    Degrees,
    /// A multiplier, shown as a percentage of natural size.
    Size,
}

impl Unit {
    /// The document's value as the field shows it.
    fn shown(self, value: f64) -> f64 {
        match self {
            Self::Percent | Self::Size => value * 100.0,
            Self::Degrees => value,
        }
    }

    /// The field's value as the document stores it — rounded to a hundredth
    /// of what is shown, so a drag writes `0.25` and not `0.25000000000000006`.
    fn stored(self, shown: f64) -> f64 {
        let shown = (shown * 100.0).round() / 100.0;
        match self {
            Self::Percent | Self::Size => shown / 100.0,
            Self::Degrees => shown,
        }
    }
}

/// One value's field, or why there is no field. `Some` on the frame it changed.
fn field(ui: &mut Ui, held: Held, unit: Unit, prefix: &str) -> Option<f64> {
    match held {
        Held::Value(value) => {
            let mut shown = unit.shown(value);
            let drag = DragValue::new(&mut shown).prefix(prefix).max_decimals(2);
            let drag = match unit {
                Unit::Percent => drag.speed(0.5).suffix("%"),
                Unit::Degrees => drag.speed(0.5).suffix("°"),
                // Floored above zero: a layer at no size is not there at all,
                // and a negative one is a flip, which is its own property.
                Unit::Size => drag.speed(0.5).range(1.0..=1000.0).suffix("%"),
            };
            ui.add(drag).changed().then(|| unit.stored(shown))
        }
        Held::Stretched { wide, tall } => {
            ui.label(RichText::new(format!("{:.0}% wide, {:.0}% tall", wide * 100.0, tall * 100.0)).weak())
                .on_hover_text("Stretched on purpose, so one scale would describe half of it — ask for a single value to replace it");
            None
        }
        Held::Animated => {
            ui.label(RichText::new("animated").weak().italics())
                .on_hover_text(
                    "This changes over the clip — nothing here will flatten it into one value",
                );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector::fixture::project;
    use scorsese_core::{AssetId, Easing, Frames, Keyframe};

    fn clip(tracks: Vec<KeyframeTrack>) -> Clip {
        let mut clip = Clip::new(ClipId::new("c"), AssetId::new("a"), Frames(0), Frames(30));
        clip.keyframes = tracks;
        clip
    }

    fn track(path: &str, points: &[(u64, f64)]) -> KeyframeTrack {
        let keyframes = points
            .iter()
            .map(|&(t, value)| Keyframe {
                t: Frames(t),
                value,
                easing: Easing::Linear,
            })
            .collect();
        KeyframeTrack::new(PropertyPath::new(path), keyframes)
    }

    /// Nothing set reads as where a layer naturally sits: unmoved, upright,
    /// its own size.
    #[test]
    fn an_untouched_clip_reads_its_resting_values() {
        let transform = Transform::of(&clip(vec![]));
        assert_eq!(transform.x, Held::Value(0.0));
        assert_eq!(transform.rotation, Held::Value(0.0));
        assert_eq!(transform.scale, Held::Value(1.0));
    }

    /// One held point is a value, and the panel says it as one — in its field,
    /// not again in the list of what animates.
    #[test]
    fn one_held_point_is_a_value_and_not_an_animation() {
        let clip = clip(vec![track(ROTATION, &[(12, 45.0)])]);
        assert_eq!(Transform::of(&clip).rotation, Held::Value(45.0));
        assert!(shown_as_value(&clip, &clip.keyframes[0]));
    }

    /// The inspector's rule: a ramp gets no field, and stays in the list.
    #[test]
    fn a_ramp_is_animated_and_offers_no_value() {
        let clip = clip(vec![track(POSITION_X, &[(0, 0.0), (29, 0.5)])]);
        assert_eq!(Transform::of(&clip).x, Held::Animated);
        assert!(!shown_as_value(&clip, &clip.keyframes[0]));
    }

    /// A scale that differs by axis is not one number, and is not called
    /// animated either — it is stretched, and says by how much.
    #[test]
    fn scale_is_one_value_only_when_both_axes_agree() {
        let even = clip(vec![
            track(SCALE_X, &[(0, 0.5)]),
            track(SCALE_Y, &[(0, 0.5)]),
        ]);
        assert_eq!(Transform::of(&even).scale, Held::Value(0.5));
        let stretched = clip(vec![track(SCALE_X, &[(0, 2.0)])]);
        assert_eq!(
            Transform::of(&stretched).scale,
            Held::Stretched {
                wide: 2.0,
                tall: 1.0
            }
        );
    }

    /// Two tracks on one property is a document where one silently never
    /// plays — not a value anybody can name.
    #[test]
    fn two_tracks_on_one_property_are_not_a_value() {
        let clip = clip(vec![
            track(ROTATION, &[(0, 10.0)]),
            track(ROTATION, &[(0, 20.0)]),
        ]);
        assert_eq!(Transform::of(&clip).rotation, Held::Animated);
    }

    #[test]
    fn a_shown_percentage_is_stored_as_a_tidy_fraction() {
        assert_eq!(Unit::Percent.stored(Unit::Percent.shown(0.25)), 0.25);
        assert_eq!(Unit::Percent.stored(33.333_333), 0.3333);
        assert_eq!(Unit::Degrees.stored(-90.0), -90.0);
    }

    /// A scale is both axes at once, and holding a value over a single held
    /// point replaces it rather than stacking a second track on the property.
    #[test]
    fn holding_scale_writes_both_axes_as_one_point_each() {
        let mut project = project();
        let c1 = ClipId::new("c1");
        hold_all(&mut project, &c1, &[SCALE_X, SCALE_Y], 0.5).expect("a size is legal");
        hold_all(&mut project, &c1, &[SCALE_X, SCALE_Y], 0.8).expect("and so is another");
        let clip = &project.tracks[0].clips[0];
        assert_eq!(clip.keyframes.len(), 2, "{:?}", clip.keyframes);
        assert_eq!(Transform::of(clip).scale, Held::Value(0.8));
    }

    /// A value the format cannot carry is refused, and the clip is untouched.
    #[test]
    fn a_value_that_is_not_a_number_is_refused() {
        let mut project = project();
        let refused = hold_all(&mut project, &ClipId::new("c1"), &[ROTATION], f64::NAN);
        assert!(refused.is_err());
        assert!(project.tracks[0].clips[0].keyframes.is_empty());
    }
}
