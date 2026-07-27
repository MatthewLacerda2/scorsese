//! What is audible, and — the finding worth having — what is not.
//!
//! A clip in the mix is not the same as a clip you can hear. A prompt nobody
//! generated contributes silence, a clip held at zero volume contributes
//! nothing, and a shot whose file has no audio stream is not in the mix at all.
//! All three read as "there is sound here" to anything that only counts clips.

use scorsese_core::{AssetKind, Easing};

use crate::common::{
    audio_track, clip, file_asset, narration_asset, project, silent_asset, sounding_asset,
    video_track,
};
use crate::{animated, described, ramp};

#[test]
fn a_clip_held_at_zero_volume_is_named_as_silent() {
    let project = project(
        vec![
            file_asset("shot", AssetKind::Video),
            sounding_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "shot", 0, 30)]),
            audio_track(
                "a1",
                vec![animated(
                    clip("m1", "music", 0, 30),
                    vec![ramp("volume", [(0, 0.0), (30, 0.0)], Easing::Linear)],
                )],
            ),
        ],
    );

    let description = described(&project);
    let music = &description.stretches[0].sound[0];

    assert_eq!(music.clip, "m1", "it is still in the mix");
    assert!(
        !music.is_audible(),
        "but nothing of it can be heard: {music:?}"
    );
    assert!(format!("{description}").contains("music (audio) — silent"));
}

/// The same clip at a real level. Without this, "silent" could be what the
/// describer says about every clip.
#[test]
fn a_clip_at_a_real_level_is_audible() {
    let project = project(
        vec![
            file_asset("shot", AssetKind::Video),
            sounding_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "shot", 0, 30)]),
            audio_track(
                "a1",
                vec![animated(
                    clip("m1", "music", 0, 30),
                    vec![ramp("volume", [(0, 0.3), (30, 0.3)], Easing::Linear)],
                )],
            ),
        ],
    );

    let description = described(&project);

    assert!(description.stretches[0].sound[0].is_audible());
    assert!(format!("{description}").contains("volume 0.30 held"));
    assert!(!format!("{description}").contains("— silent"));
}

/// A shot whose own file carries an audio stream is a sound as well as a
/// picture, and one whose file does not is only a picture. The difference is
/// read from the assets table, so a description says it without decoding
/// anything.
#[test]
fn a_shot_is_in_the_mix_only_when_its_own_file_has_sound_on_it() {
    let heard = project(
        vec![sounding_asset("shot", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "shot", 0, 30)])],
    );
    let mute = project(
        vec![silent_asset("shot", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "shot", 0, 30)])],
    );

    let heard = described(&heard);
    assert_eq!(heard.stretches[0].sound.len(), 1);
    assert!(heard.stretches[0].sound[0].is_audible());
    assert!(!heard.is_silent());

    let mute = described(&mute);
    assert!(
        mute.stretches[0].sound.is_empty(),
        "a silent file is not in the mix at all"
    );
    assert!(mute.is_silent());
    assert!(format!("{mute}").contains("no soundtrack"));
}

/// A narration prompt is in the mix and cannot be heard: its sound has not been
/// generated. A description that called it audible would be describing the film
/// somebody intends rather than the one that would come out.
#[test]
fn an_ungenerated_narration_is_in_the_mix_and_silent() {
    let project = project(
        vec![file_asset("shot", AssetKind::Video), narration_asset("vo")],
        vec![
            video_track("v1", vec![clip("c1", "shot", 0, 30)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 30)]),
        ],
    );

    let description = described(&project);
    let narration = &description.stretches[0].sound[0];

    assert_eq!(narration.clip, "vo1");
    assert!(!narration.is_audible());
    assert!(
        format!("{description}").contains("vo (card, sketch) — silent"),
        "{description}"
    );
}
