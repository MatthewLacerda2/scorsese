//! What an arrow may attach itself to.
//!
//! Three refusals, all answered from the document. The third — an arrow
//! attached to another arrow — is doing more work than it looks: it is what
//! forecloses every cycle there could be, so nothing resolving endpoints per
//! frame has to walk a graph or discover a loop at render time.

use crate::common::{assert_only_problem, asset_id, project};
use scorsese_core::{
    Asset, AssetProblem as E, Attach, ClipId, Curve, Endpoint, Geometry, Heads, Point, Rgba, Shape,
    ShapeProblem as S, Side, TrackKind,
};

/// An arrow whose `to` end follows `clip`, added to the fixture project.
fn attached_to(clip: &str) -> scorsese_core::Project {
    let mut p = project();
    let geometry = Geometry::Arrow {
        from: Point::new(0.1, 0.1).into(),
        to: Endpoint::Attached {
            attach: Attach {
                clip: ClipId::new(clip),
                side: Side::Left,
            },
        },
        curve: Curve::Straight,
        heads: Heads::End,
    };
    p.assets.push(Asset::shape(
        asset_id("a-to-b"),
        Shape::outlined(geometry, Rgba::BLACK),
    ));
    p
}

/// The id of a clip the fixture already has on a video track, and one on an
/// audio track — read off the project rather than assumed, so a change to the
/// fixture fails loudly here instead of quietly testing nothing.
fn a_clip_on(kind: TrackKind) -> String {
    project()
        .clips()
        .find(|(track, _)| track.kind == kind)
        .map(|(_, clip)| clip.id.to_string())
        .expect("the fixture has a clip on each kind of track")
}

#[test]
fn an_arrow_may_follow_a_clip_that_is_on_screen() {
    let project = attached_to(&a_clip_on(TrackKind::Video));
    assert_eq!(project.validate(), Ok(()));
}

/// The commonest way this breaks in practice: the clip was renamed or deleted
/// and the arrow was left behind.
#[test]
fn an_arrow_may_not_follow_a_clip_that_is_not_there() {
    assert_only_problem(
        &attached_to("c-nothing"),
        E::Shape(S::AttachedToNothing {
            asset: asset_id("a-to-b"),
            clip: ClipId::new("c-nothing"),
        }),
    );
}

/// Sound has no rectangle, so there is nothing about where it sits that could
/// answer where an arrow should point.
#[test]
fn an_arrow_may_not_follow_a_sound() {
    let clip = a_clip_on(TrackKind::Audio);
    assert_only_problem(
        &attached_to(&clip),
        E::Shape(S::AttachedToSound {
            asset: asset_id("a-to-b"),
            clip: ClipId::new(&clip),
        }),
    );
}

/// The rule that forecloses every cycle. An arrow following an arrow following
/// the first has no answer, and refusing the first step is cheaper and clearer
/// than discovering the loop.
#[test]
fn an_arrow_may_not_follow_another_arrow() {
    let mut p = attached_to("c-line");
    p.assets.push(Asset::shape(
        asset_id("line"),
        Shape::outlined(
            Geometry::Arrow {
                from: Point::new(0.1, 0.1).into(),
                to: Point::new(0.9, 0.9).into(),
                curve: Curve::Straight,
                heads: Heads::None,
            },
            Rgba::BLACK,
        ),
    ));
    let track = p
        .tracks
        .iter_mut()
        .find(|track| track.kind == TrackKind::Video)
        .expect("the fixture has a video track");
    let mut clip = track.clips[0].clone();
    clip.id = ClipId::new("c-line");
    clip.asset = asset_id("line");
    clip.start = scorsese_core::Frames(600);
    track.clips.push(clip);

    assert_only_problem(
        &p,
        E::Shape(S::AttachedToArrow {
            asset: asset_id("a-to-b"),
            clip: ClipId::new("c-line"),
        }),
    );
}

/// Two attached ends are never refused for coinciding. Where they land is a
/// fact about a frame, and two boxes that happen to overlap for one shot are not
/// a project that failed to load.
#[test]
fn two_attached_ends_are_not_held_to_being_apart() {
    let clip = a_clip_on(TrackKind::Video);
    let mut p = project();
    let at = |clip: &str| Endpoint::Attached {
        attach: Attach {
            clip: ClipId::new(clip),
            side: Side::Center,
        },
    };
    p.assets.push(Asset::shape(
        asset_id("a-to-b"),
        Shape::outlined(
            Geometry::Arrow {
                from: at(&clip),
                to: at(&clip),
                curve: Curve::Straight,
                heads: Heads::End,
            },
            Rgba::BLACK,
        ),
    ));
    assert_eq!(p.validate(), Ok(()));
}
