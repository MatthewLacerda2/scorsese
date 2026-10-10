use std::collections::HashSet;

use super::{Platform, STYLES, style, styles_for};

#[test]
fn every_description_is_at_most_thirty_words() {
    for style in STYLES {
        let words = style.description.split_whitespace().count();
        assert!(words <= 30, "{} has {words} words", style.id);
    }
}

#[test]
fn every_platform_has_at_least_six_styles() {
    for platform in Platform::ALL {
        let count = styles_for(platform).count();
        assert!(count >= 6, "{platform} has {count} styles");
    }
}

#[test]
fn style_ids_are_unique_and_found_by_id() {
    let mut seen = HashSet::new();
    for each in STYLES {
        assert!(seen.insert(each.id), "{} twice", each.id);
        assert_eq!(style(each.id), Some(each));
    }
    assert_eq!(style("nope"), None);
}

/// A platform list in [`Platform::ALL`]'s order, without repeats, so a style
/// lists its placements the way the menu shows them.
#[test]
fn every_style_lists_its_platforms_in_order() {
    for style in STYLES {
        let positions: Vec<_> = style
            .platforms
            .iter()
            .map(|p| Platform::ALL.iter().position(|q| q == p))
            .collect();
        assert!(
            positions.is_sorted() && !style.platforms.is_empty(),
            "{}",
            style.id
        );
        assert!(positions.windows(2).all(|w| w[0] != w[1]), "{}", style.id);
    }
}

#[test]
fn every_prompt_says_what_it_needs_from_the_person() {
    for style in STYLES {
        assert!(
            style.prompt.contains("Needs from the person:"),
            "{}",
            style.id
        );
    }
}

#[test]
fn a_platform_reads_back_from_its_id_and_serde_name() {
    for platform in Platform::ALL {
        assert_eq!(platform.id().parse(), Ok(platform));
        let json = serde_json::to_string(&platform).expect("serialises");
        assert_eq!(json, format!("\"{}\"", platform.id()));
    }
    let refused = "facebook".parse::<Platform>().expect_err("not a platform");
    assert!(refused.to_string().contains("tiktok_ad"), "{refused}");
}

#[test]
fn youtube_is_landscape_and_every_feed_upright() {
    assert_eq!(Platform::Youtube.size(), (1920, 1080));
    for platform in Platform::ALL.into_iter().skip(1) {
        assert_eq!(platform.size(), (1080, 1920), "{platform}");
    }
    let ads: Vec<_> = Platform::ALL.into_iter().filter(|p| p.is_ad()).collect();
    assert_eq!(ads.len(), 3);
}
