//! What a request's settings mean once every default is filled in, and the
//! key a render is kept under.

use scorsese_server::renders::{Ask, Settings, key};

use super::card;

fn ask(container: Option<&str>, resolution: Option<&str>) -> Ask {
    Ask {
        container: container.map(str::to_owned),
        resolution: resolution.map(str::to_owned),
        ..Ask::default()
    }
}

#[test]
fn nothing_asked_is_an_hd_mp4_and_a_sound_only_container_has_no_picture() {
    let video = Settings::from_ask(&Ask::default()).unwrap();
    assert_eq!(video.container, "mp4");
    assert_eq!(video.video_codec.as_deref(), Some("h264"));
    assert_eq!(video.audio_codec, "aac");
    assert_eq!(video.resolution.as_deref(), Some("1920x1080"));
    assert_eq!(video.content_type(), "video/mp4");

    let sound = Settings::from_ask(&ask(Some("mp3"), None)).unwrap();
    assert_eq!((&sound.video_codec, &sound.resolution), (&None, &None));
    assert_eq!(sound.audio_codec, "mp3");
    assert_eq!(sound.content_type(), "audio/mpeg");
}

#[test]
fn the_key_follows_the_document_and_the_settings_and_nothing_else() {
    let hd = Settings::from_ask(&Ask::default()).unwrap();
    let small = Settings::from_ask(&ask(None, Some("64x36"))).unwrap();
    let project = card(|_| {});
    let base = key(&project, &hd).unwrap();
    assert_eq!(base.len(), 64);
    assert_eq!(
        key(&card(|_| {}), &hd).unwrap(),
        base,
        "the same document, the same key"
    );
    assert_eq!(
        key(
            &project,
            &Settings::from_ask(&ask(Some("MP4"), None)).unwrap()
        )
        .unwrap(),
        base
    );
    assert_ne!(key(&project, &small).unwrap(), base);
    let edited = card(|document| document["assets"][0]["color"] = "#000000".into());
    assert_ne!(key(&edited, &hd).unwrap(), base);
}

#[test]
fn settings_read_back_from_a_row_are_checked_again() {
    let project = card(|_| {});
    let mut forged = Settings::from_ask(&Ask::default()).unwrap();
    assert!(forged.render(&project).is_ok());
    forged.video_codec = Some("wmv2".to_owned());
    assert!(forged.render(&project).unwrap_err().contains("wmv2"));
}
