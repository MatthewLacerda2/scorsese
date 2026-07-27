//! Narration prompts: silence in the mix, a card on the picture.
//!
//! A cut driven by a voice-over is the case slug cards exist for. Until the
//! narration is generated there is nothing to hear, and were that all, a
//! preview of such a cut would be a black screen and a shrug. So the sound is
//! silence and the prompt is on the picture, for the frames its clip covers.

use scorsese_core::{AssetKind, GenerationState};
use scorsese_render::{Note, StandIn};

use crate::common::audio::{RATE, assert_silent, has_soundtrack, level, soundtrack};
use crate::common::ffmpeg::{fixture_dir, generate, mean_rgb, tools};
use crate::common::prompts::in_state;
use crate::common::{audio_track, clip, narration_asset, project, video_track};
use crate::{picture, render};

/// A tone written where a generated narration would have landed.
fn generated_tone(tools: &scorsese_render::Tools, root: &std::path::Path, id: &str) {
    generate(
        tools,
        &root.join(format!("generated/{id}.wav")),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("aevalsrc=exprs=0.8*sin(2*PI*440*t):duration=2:sample_rate={RATE}"),
        ],
    );
}

#[test]
fn a_narration_sketch_is_silence_in_the_mix_and_a_card_on_the_picture() {
    let tools = tools();
    let dir = fixture_dir("narration-sketch");
    let with_narration = project(
        vec![picture(&tools, &dir, 2), narration_asset("vo")],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 60)]),
        ],
    );

    let (out, report) = render(&tools, &with_narration, &dir);

    assert!(report.notes.is_empty(), "a sketch is not a fault");
    assert!(
        has_soundtrack(&tools, &out),
        "a soundtrack of silence, not the absence of one — the narration lands \
         in it once it is generated"
    );
    let heard = soundtrack(&tools, &out);
    assert_silent(level(&heard, 0.1, 1.9), "an ungenerated narration");

    // The picture is a black plate, so anything at all on it is the card.
    let (red, green, blue) = mean_rgb(&tools, &out, 30);
    assert!(
        red > 0 || green > 0 || blue > 0,
        "the narration card has to be on the picture; found {red},{green},{blue}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The control for the assertion above: the same cut without the narration
/// clip is a black frame and nothing else.
#[test]
fn the_same_cut_without_a_narration_clip_shows_nothing() {
    let tools = tools();
    let dir = fixture_dir("narration-none");
    let bare = project(
        vec![picture(&tools, &dir, 2)],
        vec![video_track("v1", vec![clip("c1", "picture", 0, 60)])],
    );

    let (out, _) = render(&tools, &bare, &dir);

    assert_eq!(
        mean_rgb(&tools, &out, 30),
        (0, 0, 0),
        "nothing is drawn over a shot that has no prompt over it"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Once it exists it is heard, and nothing is drawn over the shot: the card is
/// a stand-in, not a caption.
#[test]
fn a_generated_narration_is_heard_and_leaves_the_picture_alone() {
    let tools = tools();
    let dir = fixture_dir("narration-generated");
    generated_tone(&tools, &dir, "vo");
    let project = project(
        vec![
            picture(&tools, &dir, 2),
            in_state("vo", AssetKind::GeneratedAudio, GenerationState::Generated),
        ],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 60)]),
        ],
    );

    let (out, report) = render(&tools, &project, &dir);

    assert!(report.notes.is_empty(), "nothing is missing here");
    let heard = soundtrack(&tools, &out);
    assert!(
        level(&heard, 0.1, 1.9) > 0.08,
        "a generated narration is decoded like any other sound"
    );
    assert_eq!(
        mean_rgb(&tools, &out, 30),
        (0, 0, 0),
        "and no card is drawn over the shot"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The fallback, in the mix: a narration the project believes in and cannot
/// find is silence and a note, not a dead render.
#[test]
fn a_generated_narration_that_is_gone_is_silence_and_says_so() {
    let tools = tools();
    let dir = fixture_dir("narration-gone");
    let project = project(
        vec![
            picture(&tools, &dir, 2),
            in_state("vo", AssetKind::GeneratedAudio, GenerationState::Generated),
        ],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 60)]),
        ],
    );

    let (out, report) = render(&tools, &project, &dir);

    assert_eq!(
        report.notes,
        [Note::GeneratedMissing {
            clip: "vo1".to_owned(),
            asset: "vo".to_owned(),
            stood_in: StandIn::Silence,
        }]
    );
    assert_silent(
        level(&soundtrack(&tools, &out), 0.1, 1.9),
        "a lost narration",
    );
    std::fs::remove_dir_all(&dir).ok();
}
