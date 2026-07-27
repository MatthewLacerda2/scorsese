//! Reading a plan back as a comparable value.
//!
//! A plan is a nest of segments, shots, and frame counts; a test that walks it
//! by hand asserts on the walk as much as on the plan. These flatten it into
//! one value per question, so a failure prints the whole sequencing side by
//! side with what was expected.

use scorsese_render::plan::Plan;

/// A plan as `(timeline start, timeline frames, what fills it, output frames)`
/// per segment — the whole of what sequencing decides, in one comparable value.
///
/// "What fills it" is the clips visible through the stretch, bottom track
/// first, joined by `+`; `gap` when nothing is.
pub(crate) fn shape(plan: &Plan<'_>) -> Vec<(u64, u64, String, u64)> {
    plan.segments()
        .iter()
        .map(|segment| {
            let what = if segment.is_gap() {
                "gap".to_owned()
            } else {
                segment
                    .layers
                    .iter()
                    .map(|shot| shot.clip.id.to_string())
                    .collect::<Vec<_>>()
                    .join("+")
            };
            (
                segment.start.get(),
                segment.duration.get(),
                what,
                plan.out_frames_of(segment),
            )
        })
        .collect()
}

/// The audible half of a plan as `(timeline start, timeline frames, what is
/// heard)` per stretch — the same reading as [`shape`], for sound.
pub(crate) fn audio_shape(plan: &Plan<'_>) -> Vec<(u64, u64, String)> {
    plan.audio()
        .iter()
        .map(|segment| {
            let what = if segment.is_gap() {
                "silence".to_owned()
            } else {
                segment
                    .layers
                    .iter()
                    .map(|shot| shot.clip.id.to_string())
                    .collect::<Vec<_>>()
                    .join("+")
            };
            (segment.start.get(), segment.duration.get(), what)
        })
        .collect()
}

/// Where each shot opens in its source, by clip id, in segment then track order.
pub(crate) fn source_ins(plan: &Plan<'_>) -> Vec<(String, u64)> {
    plan.segments()
        .iter()
        .flat_map(|segment| segment.layers.iter())
        .map(|shot| (shot.clip.id.to_string(), shot.source_in.get()))
        .collect()
}
