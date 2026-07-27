//! Which clips show media and which show a slug card.
//!
//! The dispatch is a function of the **document** — state and path, nothing on
//! disk — so it is pinned here, where a whole lifecycle can be sequenced
//! without ffmpeg being in the room.

use scorsese_core::{AssetKind, Fps, GenerationState};
use scorsese_render::{FrameRange, Plan, Showing};

use crate::common::prompts::in_state;
use crate::common::{
    audio_shape, audio_track, clip, file_asset, generated_asset, narration_asset, project, shape,
    sketch_asset, sounding_asset, stale_asset, video_track,
};

/// What each layer of the first segment is showing, in track order.
fn showing(project: &scorsese_core::Project) -> Vec<(String, Showing)> {
    let plan = Plan::build(project, Fps::THIRTY, FrameRange::ALL).expect("the timeline sequences");
    plan.segments()
        .first()
        .expect("a plan covers its range")
        .layers
        .iter()
        .map(|shot| (shot.clip.id.to_string(), shot.showing))
        .collect()
}

/// One video track, one clip, one asset — the shape every dispatch question
/// below reduces to.
fn one_clip(asset: scorsese_core::Asset) -> scorsese_core::Project {
    let id = asset.id.to_string();
    project(
        vec![asset],
        vec![video_track("v1", vec![clip("c1", &id, 0, 30)])],
    )
}

#[test]
fn a_sketch_shows_a_card_rather_than_stopping_the_render() {
    assert_eq!(
        showing(&one_clip(sketch_asset("veo1"))),
        [("c1".to_owned(), Showing::Card)]
    );
}

/// The subtle one. `stale` means the prompt was edited *after* generation, so
/// the file is still sitting there — and it is the wrong shot. Rendering it
/// would silently show footage the project no longer asks for.
#[test]
fn a_stale_prompt_shows_a_card_even_though_its_file_is_still_there() {
    let asset = stale_asset("veo1");
    assert!(asset.path.is_some(), "stale media is still on disk");
    assert_eq!(
        showing(&one_clip(asset)),
        [("c1".to_owned(), Showing::Card)]
    );
}

#[test]
fn a_queued_prompt_shows_a_card_while_it_is_in_flight() {
    let queued = in_state("veo1", AssetKind::GeneratedVideo, GenerationState::Queued);
    assert_eq!(
        showing(&one_clip(queued)),
        [("c1".to_owned(), Showing::Card)]
    );
}

#[test]
fn a_generated_prompt_shows_the_media_it_paid_for() {
    assert_eq!(
        showing(&one_clip(generated_asset(
            "veo1",
            AssetKind::GeneratedVideo
        ))),
        [("c1".to_owned(), Showing::Media)]
    );
}

#[test]
fn an_imported_clip_is_untouched_by_any_of_this() {
    assert_eq!(
        showing(&one_clip(file_asset("a", AssetKind::Video))),
        [("c1".to_owned(), Showing::Media)]
    );
}

/// A narration prompt is on an audio track and its card is picture: it goes
/// over the shots it narrates, so a cut driven by a voice-over can be watched
/// before a word of it has been paid for.
#[test]
fn a_narration_sketch_is_a_layer_over_the_picture() {
    let project = project(
        vec![sounding_asset("a", AssetKind::Video), narration_asset("vo")],
        vec![
            video_track("v1", vec![clip("shot", "a", 0, 30)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 30)]),
        ],
    );

    assert_eq!(
        showing(&project),
        [
            ("shot".to_owned(), Showing::Media),
            ("vo1".to_owned(), Showing::Card),
        ],
        "the card sits above the shot, not below it"
    );
    assert_eq!(
        audio_shape(&Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("sequences")),
        [(0, 30, "shot+vo1".to_owned())],
        "and it is still a clip in the mix, contributing silence"
    );
}

/// Once the narration exists, nothing is drawn over the picture at all — the
/// card is a stand-in, not a caption.
#[test]
fn a_generated_narration_puts_nothing_on_the_picture() {
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            generated_asset("vo", AssetKind::GeneratedAudio),
        ],
        vec![
            video_track("v1", vec![clip("shot", "a", 0, 30)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 30)]),
        ],
    );

    assert_eq!(showing(&project), [("shot".to_owned(), Showing::Media)]);
}

/// A narration card does not make the render longer: picture still decides
/// that, and a voice-over past the last shot is trimmed like any other sound.
#[test]
fn a_narration_card_never_outlasts_the_picture() {
    let project = project(
        vec![file_asset("a", AssetKind::Video), narration_asset("vo")],
        vec![
            video_track("v1", vec![clip("shot", "a", 0, 30)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 90)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("sequences");

    assert_eq!(shape(&plan), [(0, 30, "shot+vo1".to_owned(), 30)]);
}
