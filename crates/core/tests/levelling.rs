//! Setting one clip's property to a value: what lands in the document, and
//! what is refused before anything does.
//!
//! Asserted on the keyframes produced, like ducking, because that is the whole
//! contract — a level is a keyframe generator, and no sound has to exist for
//! the points to be right. The property is named `volume` here only because the
//! fixture animates that one; nothing under test knows what it means.

#[path = "common/mod.rs"]
mod common;

use scorsese_core::level::{self, Level, LevelError};
use scorsese_core::{ClipId, Frames, KeyframeTrack, Project, PropertyPath};

fn volume() -> PropertyPath {
    PropertyPath::new("volume")
}

/// Sets a level on a clip of the shared fixture.
fn set(project: &mut Project, clip: &str, level: Level) -> Result<Vec<KeyframeTrack>, LevelError> {
    level::set(project, &ClipId::new(clip), &volume(), level).map(|report| report.replaced)
}

/// The `(frame, value)` points of the one track animating `volume` on a clip.
fn points(project: &Project, clip: &str) -> Vec<(u64, f64)> {
    track(project, clip)
        .keyframes
        .iter()
        .map(|point| (point.t.get(), point.value))
        .collect()
}

fn track(project: &Project, clip: &str) -> KeyframeTrack {
    let (_, found) = project
        .clips()
        .find(|(_, candidate)| candidate.id.as_str() == clip)
        .expect("the fixture has that clip");
    let mut animating = found
        .keyframes
        .iter()
        .filter(|track| track.property == volume());
    let one = animating.next().expect("volume is animated").clone();
    assert!(animating.next().is_none(), "one property, one track");
    one
}

#[test]
fn a_flat_level_is_one_keyframe_held_from_the_first_frame() {
    let mut project = common::project();
    let replaced = set(&mut project, "c-score", Level::Flat(0.4)).expect("a level");
    assert!(replaced.is_empty(), "the clip animated nothing before");
    assert_eq!(points(&project, "c-score"), vec![(0, 0.4)]);
}

/// Muting is a level of zero and nothing else — no flag, no field, no second
/// mechanism to keep in step with the first.
#[test]
fn muting_is_a_level_of_zero() {
    let mut project = common::project();
    set(&mut project, "c-score", Level::Flat(0.0)).expect("a mute is a level");
    assert_eq!(points(&project, "c-score"), vec![(0, 0.0)]);
}

/// Unsigned, so `duck_music` and everything else treats it as the user's own
/// work and never replaces it.
#[test]
fn what_it_writes_is_not_signed_by_a_tool() {
    let mut project = common::project();
    set(&mut project, "c-score", Level::Flat(1.0)).expect("a level");
    assert_eq!(track(&project, "c-score").by, None);
}

#[test]
fn a_ramp_is_two_points_at_the_instants_asked_for() {
    let mut project = common::project();
    let fade = Level::Ramp {
        from: 1.0,
        to: 0.0,
        at: Frames(240),
        over: Frames(30),
    };
    set(&mut project, "c-score", fade).expect("the ramp fits");
    assert_eq!(points(&project, "c-score"), vec![(240, 1.0), (270, 0.0)]);
}

/// One property animates from one track, so setting a level takes the lane
/// over — and hands back what it displaced, which is the only way a caller can
/// say a hand-written fade or somebody's dip has gone.
#[test]
fn setting_a_level_replaces_what_animated_the_property_and_reports_it() {
    let mut project = common::project();
    let replaced = set(&mut project, "c-vo", Level::Flat(0.5)).expect("a level");

    assert_eq!(replaced.len(), 1, "the fixture's hand-written fade");
    assert_eq!(replaced[0].keyframes.len(), 2);
    assert_eq!(replaced[0].by, None);
    assert_eq!(points(&project, "c-vo"), vec![(0, 0.5)]);
}

#[test]
fn tracks_animating_anything_else_are_left_exactly_as_they_were() {
    let mut project = common::project();
    let before = common::project();
    set(&mut project, "c-logo", Level::Flat(0.2)).expect("a level");

    let animated = |project: &Project| {
        let (_, clip) = project
            .clips()
            .find(|(_, clip)| clip.id.as_str() == "c-logo")
            .expect("the fixture has it");
        clip.keyframes.clone()
    };
    let (was, now) = (animated(&before), animated(&project));
    assert_eq!(now.len(), was.len() + 1, "one new track, none lost");
    assert_eq!(now[..was.len()], was[..], "and the others untouched");
}

/// Every refusal leaves the document exactly as it was — a level is one edit,
/// and half of one is not an outcome.
///
/// Each case is pinned to the refusal it should get, not merely to *some*
/// error. A level the format cannot carry would eventually be caught by
/// validation anyway, so asserting `is_err()` alone would pass just as happily
/// with the up-front check gone — and the caller would be told their document
/// is broken rather than which number was the problem.
#[test]
fn a_refused_level_changes_nothing_and_says_which_refusal_it_was() {
    let unchanged = common::project();
    for (asked, clip, said) in [
        (Level::Flat(f64::NAN), "c-score", "must be a finite number"),
        (Level::Flat(0.5), "c-nothing", "no clip"),
        (
            Level::Ramp {
                from: 1.0,
                to: 0.0,
                at: Frames(0),
                over: Frames::ZERO,
            },
            "c-score",
            "a ramp of no frames is a jump",
        ),
        (
            Level::Ramp {
                from: 1.0,
                to: 0.0,
                at: Frames(290),
                over: Frames(30),
            },
            "c-score",
            "frames long",
        ),
    ] {
        let mut project = common::project();
        let refused = set(&mut project, clip, asked).expect_err("{asked:?} was not refused");
        assert!(
            refused.to_string().contains(said),
            "{asked:?} on {clip} came back as `{refused}`, which does not say `{said}`"
        );
        assert_eq!(project, unchanged, "{asked:?} on {clip} changed something");
    }
}

/// A fade that finishes exactly where the clip does is allowed — it is *the*
/// fade-out, and the boundary is the whole question. One frame further is
/// [`LevelError::PastTheEnd`]; here the last point lands on the clip's edge,
/// which is where a hand would drag it.
#[test]
fn a_ramp_may_finish_exactly_where_the_clip_does() {
    let mut project = common::project();
    let (_, clip) = project
        .clips()
        .find(|(_, candidate)| candidate.id.as_str() == "c-score")
        .expect("the fixture has that clip");
    let over = Frames(30);
    let at = Frames(clip.duration.get() - over.get());
    set(
        &mut project,
        "c-score",
        Level::Ramp {
            from: 1.0,
            to: 0.0,
            at,
            over,
        },
    )
    .expect("a fade ending with the clip is the ordinary one");
    assert_eq!(
        points(&project, "c-score").last(),
        Some(&(at.get() + 30, 0.0))
    );
}
