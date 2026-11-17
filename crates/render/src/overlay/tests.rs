use scorsese_compositor::{Frame, Resolution};
use scorsese_core::style::Platform;
use scorsese_core::{Fps, Project};

use super::{Overlay, OverlayError};
use crate::ask::{AskError, RenderAsk, Spelled};
use crate::format::{Container, OutputFormat};

const UPRIGHT: (u32, u32) = (540, 960);

fn drawn(platform: Platform, zones: bool) -> Frame {
    let (width, height) = UPRIGHT;
    let overlay = Overlay::new(platform, zones).expect("a feed");
    overlay.draw(Resolution::new(width, height).expect("a raster"))
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * frame.resolution().width() + x) * 4) as usize;
    frame.bytes()[at..at + 4].try_into().expect("four bytes")
}

#[test]
fn every_feed_draws_its_controls_at_the_edges_and_leaves_the_middle_clear() {
    for platform in Platform::ALL
        .into_iter()
        .filter(|p| *p != Platform::Youtube)
    {
        let frame = drawn(platform, false);
        let (width, height) = UPRIGHT;
        let inked = |rows: std::ops::Range<u32>| {
            rows.flat_map(|y| (0..width).map(move |x| (x, y)))
                .filter(|&(x, y)| pixel(&frame, x, y)[3] > 200)
                .count()
        };
        assert!(inked(0..height / 8) > 50, "{platform}: nothing at the top");
        assert!(
            inked(height * 7 / 8..height) > 500,
            "{platform}: nothing at the foot"
        );
        assert_eq!(pixel(&frame, width / 4, height * 2 / 5)[3], 0, "{platform}");
    }
}

#[test]
fn a_tint_is_the_vendors_margins_and_nothing_where_there_are_none() {
    let height = UPRIGHT.1;
    // Meta keeps 6% of each side clear: 32 pixels of 540.
    let ad = drawn(Platform::InstagramReelsAd, true);
    let side = pixel(&ad, 20, height / 2);
    assert!(
        side[0] > 200 && side[3] > 40,
        "the side band is tinted: {side:?}"
    );
    assert_eq!(pixel(&ad, 60, height / 2)[3], 0, "the clear area is not");
    assert_eq!(
        pixel(&drawn(Platform::InstagramReelsAd, false), 20, height / 2)[3],
        0
    );
    // TikTok publishes no figures, so asking draws exactly what not asking does.
    let tiktok = Overlay::new(Platform::TiktokAd, true).expect("a feed");
    assert!(!tiktok.tinted());
    assert_eq!(
        drawn(Platform::TiktokAd, true),
        drawn(Platform::TiktokAd, false)
    );
    assert!(
        tiktok.said().contains("no safe zone is tinted"),
        "{}",
        tiktok.said()
    );
    let meta = Overlay::new(Platform::InstagramReelsAd, true).expect("a feed");
    assert!(meta.said().contains("facebook.com/business/ads-guide"));
    assert!(meta.said().contains("approximation"));
}

#[test]
fn a_landscape_player_and_a_landscape_frame_are_refused() {
    assert_eq!(
        Overlay::new(Platform::Youtube, false),
        Err(OverlayError::NoInterface(Platform::Youtube))
    );
    let wide = Resolution::new(1920, 1080).expect("a raster");
    let reels = Overlay::new(Platform::InstagramReels, false).expect("a feed");
    assert_eq!(reels.fits(wide), Err(OverlayError::NotUpright(wide)));
    // Upright but short: the header and the rail would meet.
    let four_five = Resolution::new(1080, 1350).expect("a raster");
    assert_eq!(
        reels.fits(four_five),
        Err(OverlayError::NotUpright(four_five))
    );
    assert_eq!(
        reels.fits(Resolution::new(1080, 1920).expect("a raster")),
        Ok(())
    );
}

fn asked(spelled: Spelled<'_>, aspect: &str) -> Result<crate::RenderSettings, AskError> {
    let mut project = Project::new("overlaid", Fps::THIRTY);
    project.aspect = aspect.parse().expect("a shape");
    let format = OutputFormat::defaults_for(Container::Mp4);
    RenderAsk::read(&spelled)?.settings(format, &project)
}

#[test]
fn a_render_asks_for_one_by_platform_and_carries_it_in_its_settings() {
    let spelled = Spelled {
        overlay: Some("tiktok_ad"),
        safe_zones: Some(true),
        ..Spelled::default()
    };
    let settings = asked(spelled, "9:16").expect("upright");
    let wanted = Overlay::new(Platform::TiktokAd, true).expect("a feed");
    assert_eq!(settings.overlay, Some(wanted));
    assert_eq!(
        asked(Spelled::default(), "9:16").expect("plain").overlay,
        None
    );
}

#[test]
fn a_render_refuses_zones_alone_a_wide_frame_and_a_sound_only_file() {
    let zones = Spelled {
        safe_zones: Some(true),
        ..Spelled::default()
    };
    assert_eq!(
        asked(zones, "9:16"),
        Err(AskError::Overlay(OverlayError::ZonesWithoutOverlay))
    );
    let reels = Spelled {
        overlay: Some("instagram_reels"),
        ..Spelled::default()
    };
    assert!(matches!(
        asked(reels, "16:9"),
        Err(AskError::Overlay(OverlayError::NotUpright(_)))
    ));
    let unknown = Spelled {
        overlay: Some("myspace"),
        ..Spelled::default()
    };
    assert!(
        asked(unknown, "9:16")
            .expect_err("no such")
            .to_string()
            .starts_with("overlay: ")
    );
    let ask = RenderAsk::read(&reels).expect("read");
    let mp3 = OutputFormat::defaults_for(Container::Mp3);
    assert!(
        ask.check(mp3)
            .expect_err("no picture")
            .to_string()
            .contains("mp3")
    );
}
