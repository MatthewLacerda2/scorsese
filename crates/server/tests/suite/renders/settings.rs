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

#[test]
fn a_preview_is_its_quality_of_the_delivery_size_and_a_render_is_unmarked() {
    use scorsese_server::renders::PreviewAsk;
    let preview = |resolution: Option<&str>, quality: Option<&str>| {
        Settings::from_preview(&PreviewAsk {
            resolution: resolution.map(str::to_owned),
            quality: quality.map(str::to_owned),
        })
    };
    let half = preview(None, None).unwrap();
    assert_eq!(half.resolution.as_deref(), Some("960x540"));
    assert_eq!(
        (half.container.as_str(), half.preview.as_deref()),
        ("mp4", Some("half"))
    );
    let quarter = preview(Some("1080x1920"), Some("quarter")).unwrap();
    assert_eq!(quarter.resolution.as_deref(), Some("270x480"));
    assert!(
        preview(None, Some("low"))
            .unwrap_err()
            .contains("full, half or quarter")
    );

    // A finished render's stored form has no `preview` at all, so its key is
    // the one it had before previews existed.
    let render = serde_json::to_value(Settings::from_ask(&Ask::default()).unwrap()).unwrap();
    assert!(render.get("preview").is_none(), "{render}");
    let project = card(|_| {});
    let same_size = Settings::from_ask(&ask(None, Some("960x540"))).unwrap();
    assert_ne!(
        key(&project, &half).unwrap(),
        key(&project, &same_size).unwrap()
    );
}

#[test]
fn narration_bands_left_out_are_kept_in_the_key_and_drawn_bands_are_not() {
    let project = card(|_| {});
    let drawn = Settings::from_ask(&Ask::default()).unwrap();
    assert!(drawn.narration_bands);
    // Drawn is the default, and left out of the stored form: every render
    // asked for before the choice existed keeps its key.
    let stored = serde_json::to_value(&drawn).unwrap();
    assert!(stored.get("narration_bands").is_none(), "{stored}");
    let asked_drawn = Ask {
        narration_bands: Some(true),
        ..Ask::default()
    };
    let same = Settings::from_ask(&asked_drawn).unwrap();
    assert_eq!(
        key(&project, &same).unwrap(),
        key(&project, &drawn).unwrap()
    );

    let omitted = Settings::from_ask(&Ask {
        narration_bands: Some(false),
        ..Ask::default()
    })
    .unwrap();
    assert!(!omitted.narration_bands);
    assert_ne!(
        key(&project, &omitted).unwrap(),
        key(&project, &drawn).unwrap()
    );
    let back: Settings = serde_json::from_value(serde_json::to_value(&omitted).unwrap()).unwrap();
    assert_eq!(back, omitted, "a stored row reads back as it was asked");
    assert!(omitted.render(&project).is_ok());

    let sound = Settings::from_ask(&Ask {
        narration_bands: Some(false),
        ..ask(Some("wav"), None)
    });
    assert!(
        sound.unwrap_err().contains("leaving narration bands out"),
        "refused in the stdio tool's words"
    );
}
