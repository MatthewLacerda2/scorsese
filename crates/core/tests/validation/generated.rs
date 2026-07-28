//! Keyframe tracks that a tool wrote, and the one rule about them: a tool
//! replaces its own work and nobody else's.

use crate::common::{assert_only_problem, asset_id, clip_id, project};
use scorsese_core::{Clip, Frames, Keyframe, KeyframeTrack, PropertyPath, ValidationError as E};

/// A two-point volume ramp on `property`, signed or not.
fn ramp(property: &str) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        vec![
            Keyframe {
                t: Frames(0),
                value: 1.0,
                easing: scorsese_core::Easing::Linear,
            },
            Keyframe {
                t: Frames(9),
                value: 0.25,
                easing: scorsese_core::Easing::EaseOut,
            },
        ],
    )
}

/// A clip animating a hand-written opacity ramp and a `duck`-written volume
/// ramp — the arrangement the whole field exists for.
fn mixed() -> Clip {
    let mut clip = Clip::new(clip_id("c"), asset_id("bed"), Frames(0), Frames(300));
    clip.keyframes = vec![ramp("opacity"), ramp("volume").generated_by("duck")];
    clip
}

#[test]
fn a_track_is_unsigned_unless_a_tool_signs_it() {
    let track = ramp("volume");
    assert!(track.by.is_none(), "hand-written by default");
    assert!(!track.is_generated_by("duck"));

    let signed = track.generated_by("duck");
    assert!(signed.is_generated_by("duck"));
    assert!(!signed.is_generated_by("fade"), "one tool, not any tool");
}

/// Replacing swaps one track for one track — it must not accumulate, which is
/// the failure a naive "just push mine" would have.
#[test]
fn a_tool_replaces_its_own_work_rather_than_adding_to_it() {
    let mut clip = mixed();
    for _ in 0..3 {
        clip.replace_generated("duck", vec![ramp("volume").generated_by("duck")]);
    }
    assert_eq!(clip.keyframes.len(), 2, "one replaced, one untouched");
    assert_eq!(
        clip.keyframes
            .iter()
            .filter(|track| track.is_generated_by("duck"))
            .count(),
        1,
        "three runs must leave one duck, not three"
    );
}

/// The rule that matters. Everything else here is bookkeeping; this is the
/// thing that stops a re-run destroying an edit.
#[test]
fn a_tool_never_touches_what_it_did_not_write() {
    let mut clip = mixed();
    let by_hand: Vec<_> = clip.hand_written().cloned().collect();
    assert_eq!(by_hand.len(), 1, "the opacity ramp is nobody's tool's");

    clip.replace_generated("duck", vec![]);
    assert_eq!(
        clip.hand_written().cloned().collect::<Vec<_>>(),
        by_hand,
        "withdrawing a duck must leave the hand-written ramp exactly as it was"
    );
}

/// A different tool's tracks are as untouchable as a person's.
#[test]
fn one_tool_does_not_replace_another_tools_work() {
    let mut clip = mixed();
    clip.keyframes.push(ramp("volume").generated_by("fade"));
    clip.replace_generated("duck", vec![]);
    assert_eq!(clip.keyframes.len(), 2);
    assert!(clip.keyframes.iter().any(|t| t.is_generated_by("fade")));
}

/// An empty replacement is how a tool withdraws, and it must actually leave.
#[test]
fn a_tool_can_withdraw_entirely() {
    let mut clip = mixed();
    clip.replace_generated("duck", vec![]);
    assert!(!clip.keyframes.iter().any(|t| t.is_generated_by("duck")));
}

#[test]
fn a_blank_signature_is_refused() {
    let mut p = project();
    let clip = &mut p.tracks[1].clips[0];
    clip.keyframes.push(KeyframeTrack {
        by: Some("   ".to_owned()),
        ..ramp("opacity")
    });
    assert_only_problem(
        &p,
        &E::BlankKeyframeAuthor {
            clip: clip_id("c-logo"),
            property: PropertyPath::new("opacity"),
        },
    );
}

/// The field must not reach anything that renders. A signed track and an
/// unsigned one with the same points are the same animation.
#[test]
fn a_signature_changes_no_value() {
    let plain = ramp("volume");
    let signed = ramp("volume").generated_by("duck");
    for frame in [0, 3, 9, 40] {
        assert_eq!(
            plain.value_at(Frames(frame)),
            signed.value_at(Frames(frame))
        );
    }
}
