//! Running it twice — the half that decides whether the feature is usable.

use crate::{dip, points, scored};
use scorsese_core::{
    Dip, Easing, Frames, Keyframe, KeyframeTrack, Project, PropertyPath, TrackId, Under, duck_track,
};

fn under() -> Under {
    Under {
        music: TrackId::new("music"),
        narration: vec![],
    }
}

fn duck(project: &mut Project, dip: Dip) {
    duck_track(project, &under(), &PropertyPath::new("volume"), dip)
        .expect("the music track exists");
}

/// A hand-written fade-in on the music, the thing a re-run must never harm.
fn add_fade(project: &mut Project) -> KeyframeTrack {
    let fade = KeyframeTrack::new(
        PropertyPath::new("volume"),
        vec![
            Keyframe {
                t: Frames(0),
                value: 0.0,
                easing: Easing::EaseIn,
            },
            Keyframe {
                t: Frames(30),
                value: 1.0,
                easing: Easing::Linear,
            },
        ],
    );
    project.tracks[0].clips[0].keyframes.push(fade.clone());
    fade
}

#[test]
fn running_twice_leaves_one_dip() {
    let mut project = scored(&[(60, 90)]);
    duck(&mut project, dip());
    let first = points(&project);
    duck(&mut project, dip());

    assert_eq!(
        points(&project),
        first,
        "the same run twice is the same dip"
    );
    assert_eq!(
        project.tracks[0].clips[0].keyframes.len(),
        1,
        "and one track, not two"
    );
}

#[test]
fn re_running_with_new_settings_replaces_the_old_dip() {
    let mut project = scored(&[(60, 90)]);
    duck(&mut project, dip());
    duck(
        &mut project,
        Dip {
            under: 0.5,
            ..dip()
        },
    );
    let values: Vec<f64> = points(&project).iter().map(|(_, value)| *value).collect();
    assert!(values.contains(&0.5), "the new depth is there");
    assert!(!values.contains(&0.25), "and the old one is gone");
}

/// The rule the whole `by` field exists for.
#[test]
fn a_hand_written_ramp_survives_every_run() {
    let mut project = scored(&[(60, 90)]);
    let fade = add_fade(&mut project);
    for _ in 0..3 {
        duck(&mut project, dip());
    }
    let hand: Vec<_> = project.tracks[0].clips[0].hand_written().collect();
    assert_eq!(hand, vec![&fade], "untouched, three runs later");
}

/// If the narration moves away, the old dip is a ramp the project no longer
/// means — so it goes, rather than being left behind for someone to find.
#[test]
fn narration_moving_away_withdraws_the_dip() {
    let mut project = scored(&[(60, 90)]);
    let fade = add_fade(&mut project);
    duck(&mut project, dip());
    assert!(!points(&project).is_empty());

    project.tracks[1].clips.clear();
    duck(&mut project, dip());
    assert!(points(&project).is_empty(), "the dip is withdrawn");
    assert_eq!(
        project.tracks[0].clips[0].keyframes,
        vec![fade],
        "and the hand-written fade is all that is left"
    );
}

#[test]
fn a_track_that_is_not_there_is_reported_rather_than_ignored() {
    let mut project = scored(&[(60, 90)]);
    let missing = Under {
        music: TrackId::new("nope"),
        narration: vec![],
    };
    assert!(
        duck_track(&mut project, &missing, &PropertyPath::new("volume"), dip()).is_none(),
        "a typo'd track name must not silently do nothing"
    );
}

/// `--under` narrows what counts as narration; anything not named is ignored
/// even though it is audible.
#[test]
fn naming_the_narration_tracks_excludes_the_others() {
    let mut project = scored(&[(60, 90)]);
    project
        .tracks
        .push(crate::audio_track("sfx", vec![crate::clip("s1", 300, 60)]));

    duck_track(
        &mut project,
        &Under {
            music: TrackId::new("music"),
            narration: vec![TrackId::new("vo")],
        },
        &PropertyPath::new("volume"),
        dip(),
    )
    .expect("the music track exists");

    assert_eq!(points(&project).len(), 4, "one dip, from `vo` alone");
}
