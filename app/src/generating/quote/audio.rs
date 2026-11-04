//! A line, a track and a sound are quoted under their own headings, so a
//! sound effect is never listed as narration at the moment money is agreed
//! to (#1265).

use scorsese_core::{
    Asset, AssetId, AssetKind, AudioBlock, Fps, MusicRequest, Project, SoundRequest, SpeechRequest,
};
use scorsese_providers::credentials::Settings;

use super::Quote;

fn audio(id: &str) -> Asset {
    Asset::sketch(AssetId::new(id), AssetKind::GeneratedAudio, "something")
}

#[test]
fn each_audio_block_is_quoted_under_its_own_heading() {
    let mut project = Project::new("audio", Fps::THIRTY);
    let mut line = audio("vo");
    line.speech = Some(SpeechRequest {
        voice_id: Some(String::from("a-voice")),
        ..SpeechRequest::default()
    });
    let mut bed = audio("bed");
    bed.music = Some(MusicRequest {
        seconds: Some(40),
        ..MusicRequest::default()
    });
    let mut door = audio("door");
    door.sound = Some(SoundRequest {
        seconds: Some(2.5),
        ..SoundRequest::default()
    });
    project.assets.extend([line, bed, door]);
    let dir = crate::scratch::fresh("scorsese-app-quote-audio", ".scor");
    std::fs::create_dir_all(dir.join("generated")).expect("create the project directory");

    let quote = Quote::of(&project, &dir, &Settings::default());
    let under = |block| -> Vec<String> {
        quote
            .of_audio(&project, block)
            .into_iter()
            .map(|item| item.subject.clone())
            .collect()
    };
    assert_eq!(under(AudioBlock::Speech), ["vo"]);
    assert_eq!(under(AudioBlock::Music), ["bed"]);
    assert_eq!(under(AudioBlock::Sound), ["door"]);
}
