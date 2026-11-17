//! A render asked for with a feed's interface over it (#1010): kept in the
//! key when asked, left out of the stored form when not, and refused in the
//! stdio tool's words.

use scorsese_server::renders::{Ask, Settings, key};

use super::card;

fn upright(overlay: Option<&str>, safe_zones: Option<bool>) -> Ask {
    Ask {
        resolution: Some("1080x1920".to_owned()),
        overlay: overlay.map(str::to_owned),
        safe_zones,
        ..Ask::default()
    }
}

#[test]
fn an_overlay_is_kept_in_the_key_and_none_leaves_it_as_it_was() {
    let project = card(|_| {});
    let plain = Settings::from_ask(&upright(None, None)).unwrap();
    let stored = serde_json::to_value(&plain).unwrap();
    assert!(stored.get("overlay").is_none(), "{stored}");
    assert!(stored.get("safe_zones").is_none(), "{stored}");

    let tinted = Settings::from_ask(&upright(Some("instagram_reels_ad"), Some(true))).unwrap();
    assert_eq!(tinted.overlay.as_deref(), Some("instagram_reels_ad"));
    assert!(tinted.safe_zones);
    assert_ne!(
        key(&project, &tinted).unwrap(),
        key(&project, &plain).unwrap()
    );
    let back: Settings = serde_json::from_value(serde_json::to_value(&tinted).unwrap()).unwrap();
    assert_eq!(back, tinted, "a stored row reads back as it was asked");
    let settings = tinted.render(&project).expect("upright");
    assert!(settings.overlay.is_some_and(|overlay| overlay.safe_zones()));
    // TikTok publishes no figures: asking for zones draws the same frames,
    // so it is the same render.
    let untinted = Settings::from_ask(&upright(Some("tiktok_ad"), Some(true))).unwrap();
    let plain_tiktok = Settings::from_ask(&upright(Some("tiktok_ad"), None)).unwrap();
    assert_eq!(
        key(&project, &untinted).unwrap(),
        key(&project, &plain_tiktok).unwrap()
    );
}

#[test]
fn zones_alone_a_player_and_a_sound_file_are_refused() {
    let refused = |ask: Ask| Settings::from_ask(&ask).unwrap_err();
    assert!(refused(upright(None, Some(true))).starts_with("safe_zones: "));
    assert!(refused(upright(Some("youtube"), None)).starts_with("overlay: "));
    let wide = Ask {
        resolution: Some("1920x1080".to_owned()),
        ..upright(Some("tiktok"), None)
    };
    assert!(refused(wide).contains("9:16"));
    let sound = Ask {
        container: Some("wav".to_owned()),
        overlay: Some("tiktok".to_owned()),
        ..Ask::default()
    };
    assert!(refused(sound).contains("an overlay"));
}
