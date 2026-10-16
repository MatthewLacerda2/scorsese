//! The choices a render takes beside its size and container (#1070): a frame
//! rate, a platform that sizes it and a stretch of the timeline — read by the
//! same rules `scorsese render` and the MCP `render` tool read them by.

use scorsese_render::FrameRange;
use scorsese_server::renders::{Ask, Settings, key};

use super::card;

fn ask(fill: impl FnOnce(&mut Ask)) -> Result<Settings, String> {
    let mut asked = Ask::default();
    fill(&mut asked);
    Settings::from_ask(&asked)
}

#[test]
fn a_frame_rate_is_kept_rendered_at_and_refused_for_sound_only() {
    let project = card(|_| {});
    let sixty = ask(|a| a.fps = Some("60".to_owned())).unwrap();
    assert_eq!(sixty.fps.as_deref(), Some("60"));
    assert_eq!(sixty.render(&project).unwrap().fps.to_string(), "60");
    let own = ask(|_| {}).unwrap();
    assert_eq!(own.render(&project).unwrap().fps, project.timeline_fps);
    assert_ne!(key(&project, &sixty).unwrap(), key(&project, &own).unwrap());

    let sound = ask(|a| {
        a.container = Some("mp3".to_owned());
        a.fps = Some("30".to_owned());
    });
    assert!(
        sound.unwrap_err().contains("mp3"),
        "a frame rate on sound only"
    );
    let decimal = ask(|a| a.fps = Some("29.97".to_owned()));
    assert!(decimal.unwrap_err().starts_with("fps: "));
}

#[test]
fn a_platform_sizes_the_render_and_cannot_be_given_beside_a_size() {
    let tiktok = ask(|a| a.platform = Some("tiktok".to_owned())).unwrap();
    assert_eq!(tiktok.resolution.as_deref(), Some("1080x1920"));
    // Kept as the size it delivers at: the same file as asking for the size.
    let sized = ask(|a| a.resolution = Some("1080x1920".to_owned())).unwrap();
    assert_eq!(tiktok, sized);
    let tiktok_ask = Ask {
        platform: Some("tiktok".to_owned()),
        ..Ask::default()
    };
    assert!(!tiktok_ask.leaves_size(&tiktok), "the platform sized it");
    assert!(Ask::default().leaves_size(&ask(|_| {}).unwrap()));

    let both = ask(|a| {
        a.platform = Some("youtube".to_owned());
        a.resolution = Some("640x360".to_owned());
    });
    assert!(both.unwrap_err().contains("give one"));
    let sound = ask(|a| {
        a.container = Some("wav".to_owned());
        a.platform = Some("tiktok".to_owned());
    });
    assert!(sound.is_err(), "a platform on sound only");
    assert!(
        ask(|a| a.platform = Some("myspace".to_owned()))
            .unwrap_err()
            .starts_with("platform: ")
    );
}

#[test]
fn a_range_is_kept_read_back_and_left_out_of_the_key_when_whole() {
    let project = card(|_| {});
    let whole = ask(|_| {}).unwrap();
    assert_eq!(whole.range(), Ok(FrameRange::ALL));
    let stored = serde_json::to_value(&whole).unwrap();
    assert!(stored.get("range").is_none(), "{stored}");
    assert!(stored.get("fps").is_none(), "{stored}");

    let part = ask(|a| a.range = Some(" 30 : ".to_owned())).unwrap();
    assert_eq!(
        part.range.as_deref(),
        Some("30:"),
        "kept as FrameRange writes it"
    );
    assert_eq!(part.range().unwrap().to_string(), "30:");
    let back: Settings = serde_json::from_value(serde_json::to_value(&part).unwrap()).unwrap();
    assert_eq!(back, part);
    assert_ne!(
        key(&project, &part).unwrap(),
        key(&project, &whole).unwrap()
    );

    // Sound has frames too: a stretch of the mix is a range of the timeline.
    let sound = ask(|a| {
        a.container = Some("mp3".to_owned());
        a.range = Some(":120".to_owned());
    });
    assert_eq!(sound.unwrap().range.as_deref(), Some("0:120"));
    let backwards = ask(|a| a.range = Some("90:30".to_owned()));
    assert!(backwards.unwrap_err().starts_with("range: "));
}
