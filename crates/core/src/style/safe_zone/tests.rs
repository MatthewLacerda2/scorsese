use super::{Edge, SafeZone};
use crate::style::Platform;

/// What each vendor's own page said on the day it was read: Meta publishes
/// margins for its two ad placements, and nobody publishes figures for the
/// rest.
#[test]
fn only_metas_ads_have_published_margins() {
    for platform in Platform::ALL {
        let zone = platform.safe_zone();
        let margins = zone.margins.map(|m| (m.top, m.bottom, m.left, m.right));
        let expected = matches!(
            platform,
            Platform::InstagramReelsAd | Platform::InstagramStoriesAd
        )
        .then_some((0.14, 0.35, 0.06, 0.06));
        assert_eq!(margins, expected, "{platform}");
        assert!(zone.source.page.starts_with("https://"), "{platform}");
        assert_eq!(zone.source.read, "2026-11-16", "{platform}");
        assert!(!zone.source.says.is_empty(), "{platform}");
    }
}

#[test]
fn the_covered_bands_and_the_clear_area_tile_the_frame() {
    let margins = Platform::InstagramReelsAd
        .safe_zone()
        .margins
        .expect("published");
    let clear = margins.clear();
    let mut total = clear.width * clear.height;
    for band in margins.covered() {
        assert!(band.area.width > 0.0 && band.area.height > 0.0, "{band:?}");
        assert!(!band.area.overlaps(&clear), "{band:?}");
        total += band.area.width * band.area.height;
    }
    assert!((total - 1.0).abs() < 1e-6, "{total}");
    let edges = margins.covered().map(|band| band.edge);
    assert_eq!(edges, [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right]);
    assert!((clear.x - 0.06).abs() < 1e-6 && (clear.y - 0.14).abs() < 1e-6);
}

/// Rounded inward, so a box inside the pixels is inside the zone.
#[test]
fn the_clear_area_in_pixels_rounds_inward() {
    let margins = Platform::InstagramStoriesAd
        .safe_zone()
        .margins
        .expect("published");
    assert_eq!(margins.clear_pixels(1080, 1920), (65, 269, 1015, 1248));
}

#[test]
fn a_placement_without_figures_says_why() {
    let SafeZone { margins, source } = Platform::TiktokAd.safe_zone();
    assert!(margins.is_none());
    assert!(source.page.contains("ads.tiktok.com"), "{}", source.page);
    assert!(source.says.contains("files"), "{}", source.says);
    let shorts = Platform::YoutubeShorts.safe_zone().source;
    assert!(shorts.says.contains("288"), "{}", shorts.says);
}

/// The guide's table states the same figures as the code, row for row.
#[test]
fn the_styles_guide_states_the_same_margins() {
    const GUIDE: &str = include_str!("../../../../../docs/styles.md");
    for platform in Platform::ALL {
        let start = format!("| `{}` |", platform.id());
        let row = GUIDE
            .lines()
            .find(|line| line.starts_with(&start))
            .unwrap_or_else(|| panic!("no row for {platform} in docs/styles.md"));
        let cells: Vec<&str> = row.split('|').map(str::trim).skip(2).take(4).collect();
        let expected: Vec<String> = match platform.safe_zone().margins {
            Some(m) => [m.top, m.bottom, m.left, m.right].map(percent).to_vec(),
            None => vec!["—".to_owned(); 4],
        };
        assert_eq!(cells, expected, "{platform}");
    }
}

/// An upright placement's brief says where its interface covers the frame,
/// in this render's pixels when the vendor publishes figures.
#[test]
fn an_upright_placements_brief_says_where_its_interface_covers_the_frame() {
    let said = |id| crate::style::recipe(id).expect("a placement");
    let reels_ad = said("instagram_reels_ad");
    assert!(reels_ad.contains("x 65-1015, y 269-1248"), "{reels_ad}");
    assert!(reels_ad.contains("read 2026-11-16"), "{reels_ad}");
    let tiktok = said("tiktok");
    assert!(tiktok.contains("only as downloadable files"), "{tiktok}");
    assert!(!said("youtube").contains("Safe zones"));
}

/// A fraction as the guide writes it: `14%`.
fn percent(fraction: f32) -> String {
    format!("{}%", (f64::from(fraction) * 100.0).round())
}
