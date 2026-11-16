use super::{Area, Edge, Margins, SafeZone};
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

/// Top and bottom across the whole width, the sides between them, so the
/// four bands and the clear area tile the frame. Uneven margins, so a band
/// measured from the wrong edge shows.
#[test]
fn the_covered_bands_and_the_clear_area_tile_the_frame() {
    let margins = Margins {
        top: 0.1,
        bottom: 0.3,
        left: 0.05,
        right: 0.2,
        covered_by: "",
    };
    let area = |x, y, width, height| Area {
        x,
        y,
        width,
        height,
    };
    let expected = [
        (Edge::Top, area(0.0, 0.0, 1.0, 0.1)),
        (Edge::Bottom, area(0.0, 0.7, 1.0, 0.3)),
        (Edge::Left, area(0.0, 0.1, 0.05, 0.6)),
        (Edge::Right, area(0.8, 0.1, 0.2, 0.6)),
    ];
    for (band, (edge, want)) in margins.covered().into_iter().zip(expected) {
        assert_eq!(band.edge, edge);
        assert_near(band.area, want);
    }
    assert_near(margins.clear(), area(0.05, 0.1, 0.75, 0.6));
}

fn assert_near(got: Area, want: Area) {
    let near = |a: f32, b: f32| (a - b).abs() < 1e-6;
    let pairs = [
        (got.x, want.x),
        (got.y, want.y),
        (got.width, want.width),
        (got.height, want.height),
    ];
    assert!(
        pairs.iter().all(|&(a, b)| near(a, b)),
        "{got:?}, not {want:?}"
    );
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
