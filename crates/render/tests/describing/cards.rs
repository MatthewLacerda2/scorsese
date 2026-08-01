//! What a stretch of slug cards says it is.
//!
//! A preview cut made entirely of prompt clips is the case this feature most
//! serves — and the one that most looks like a finished edit to anything
//! counting frames. So a card has to be named as a card, with the prompt it
//! carries and the lifecycle state it is in.

use scorsese_core::{AssetKind, GenerationState};
use scorsese_render::{Absent, Shown};

use crate::common::prompts::{NARRATION_PROMPT, SHOT_PROMPT, in_state};
use crate::common::{audio_track, clip, file_asset, project, stale_asset, text_asset, video_track};
use crate::described;

/// One clip, one asset, one stretch — the shape every question here reduces to.
fn one(asset: scorsese_core::Asset) -> scorsese_core::Project {
    let id = asset.id.to_string();
    project(
        vec![asset],
        vec![video_track("v1", vec![clip("c1", &id, 0, 30)])],
    )
}

#[test]
fn a_sketch_is_a_card_carrying_its_prompt_and_its_state() {
    let asset = in_state("veo1", AssetKind::GeneratedVideo, GenerationState::Sketch);
    let description = described(&one(asset));

    assert_eq!(
        description.stretches[0].picture[0].shows,
        Shown::Card {
            absent: Absent::Sketch,
            prompt: SHOT_PROMPT.to_owned(),
        }
    );
    assert!(
        format!("{description}").contains(&format!("veo1 (card, sketch) \"{SHOT_PROMPT}\"")),
        "{description}"
    );
}

/// The subtle one. `stale` means the prompt was edited after generation, so the
/// file is still on disk and is the wrong shot. A description that read "is
/// there a file?" would call this media.
#[test]
fn a_stale_prompt_is_a_card_even_though_its_file_is_on_disk() {
    let asset = stale_asset("veo1");
    assert!(asset.path.is_some(), "stale media is still there");

    let description = described(&one(asset));

    assert!(matches!(
        description.stretches[0].picture[0].shows,
        Shown::Card {
            absent: Absent::Stale,
            ..
        }
    ));
    assert!(format!("{description}").contains("(card, stale)"));
}

#[test]
fn a_generated_prompt_is_the_media_it_paid_for() {
    let asset = in_state(
        "veo1",
        AssetKind::GeneratedVideo,
        GenerationState::Generated,
    );
    let description = described(&one(asset));

    assert_eq!(description.stretches[0].picture[0].shows, Shown::Media);
    assert!(format!("{description}").contains("veo1 (generated_video, fit)"));
}

/// A narration prompt is a card on the picture as well as silence in the mix,
/// and both halves have to be said: the card is what a viewer sees, the silence
/// is what they do not hear.
#[test]
fn a_narration_prompt_is_a_card_on_the_picture_and_silence_under_it() {
    let project = project(
        vec![
            file_asset("shot", AssetKind::Video),
            in_state("vo", AssetKind::GeneratedAudio, GenerationState::Sketch),
        ],
        vec![
            video_track("v1", vec![clip("c1", "shot", 0, 30)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 30)]),
        ],
    );

    let description = described(&project);
    let picture: Vec<&str> = description.stretches[0]
        .picture
        .iter()
        .map(|playing| playing.clip.as_str())
        .collect();

    assert_eq!(
        picture,
        ["c1", "vo1"],
        "the card sits above the shot it narrates"
    );
    assert!(
        format!("{description}").contains(&format!("vo (card, sketch) \"{NARRATION_PROMPT}\""))
    );
    assert!(!description.stretches[0].sound[0].is_audible());
}

/// Text carries its content inline, so a description can quote it rather than
/// naming a file nobody can look at.
#[test]
fn a_text_asset_is_described_by_what_it_says() {
    let description = described(&one(text_asset("title")));

    assert_eq!(
        description.stretches[0].picture[0].shows,
        Shown::Text("THE END".to_owned())
    );
    assert!(format!("{description}").contains("title (text, fit) \"THE END\""));
}

/// A synthesised asset carries a **recipe**, not a prompt. Asking it for one
/// and finding none used to put "(no prompt)" on its card, which reports a
/// well-formed asset as broken in the one place a person looks to find out
/// what a shot is going to be.
///
/// Found by rendering the window's panels offscreen and *looking* at them —
/// nothing had ever drawn a slug card for a synth asset before.
#[test]
fn a_synth_card_names_its_recipe_rather_than_a_prompt_it_never_had() {
    use scorsese_core::{Asset, AssetId, AssetKind, ProjectPath};

    let asset = Asset::synth(
        AssetId::new("bed"),
        AssetKind::SynthAudio,
        ProjectPath::new("recipes/bed.json"),
    );
    let wording = scorsese_render::wording(&asset, Absent::Sketch);
    assert!(wording.contains("RECIPE"), "got {wording}");
    assert!(wording.contains("recipes/bed.json"), "got {wording}");
    assert!(!wording.contains("no prompt"), "got {wording}");
}
