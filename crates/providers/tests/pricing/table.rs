//! The rates themselves, and the arithmetic over them.

use scorsese_core::{ClipSeconds, VideoModel, VideoRequest, VideoResolution};
use scorsese_providers::prices::{Quality, Tier, dollars, estimate, veo};

/// A request for a shot at this tier, size and length.
fn asking(model: VideoModel, resolution: VideoResolution, seconds: ClipSeconds) -> VideoRequest {
    VideoRequest {
        model,
        resolution,
        seconds,
        ..VideoRequest::default()
    }
}

/// Every figure on Google's page, as of the date the table records. Written out
/// here rather than looped over the table, because a test that derives its
/// expectations from the thing under test proves only that it is consistent
/// with itself.
#[test]
fn the_table_says_what_the_vendors_page_says() {
    let cents = |tier, quality| veo::rate(tier, quality).map(|rate| rate.cents_per_second);
    assert_eq!(cents(Tier::Standard, Quality::P720), Some(40));
    assert_eq!(cents(Tier::Standard, Quality::P1080), Some(40));
    assert_eq!(cents(Tier::Standard, Quality::P4k), Some(60));
    assert_eq!(cents(Tier::Fast, Quality::P720), Some(10));
    assert_eq!(cents(Tier::Fast, Quality::P1080), Some(12));
    assert_eq!(cents(Tier::Fast, Quality::P4k), Some(30));
    assert_eq!(cents(Tier::Lite, Quality::P720), Some(5));
    assert_eq!(cents(Tier::Lite, Quality::P1080), Some(8));
}

/// The combination nobody can buy has no row, so there is no number to misread.
#[test]
fn lite_at_4k_is_absent_rather_than_free() {
    assert_eq!(veo::rate(Tier::Lite, Quality::P4k), None);
}

#[test]
fn the_default_shot_costs_ninety_six_cents() {
    let priced = estimate(&VideoRequest::default()).expect("Fast at 1080p is sold");
    assert_eq!(priced.cents, 96, "eight seconds at 12 cents");
    assert_eq!(priced.seconds, 8);
    assert_eq!(priced.rate.cents_per_second, 12);
}

/// The one generation this feature was tested against really did cost this.
#[test]
fn four_seconds_of_lite_at_720p_is_twenty_cents() {
    let priced = asking(VideoModel::Lite, VideoResolution::P720, ClipSeconds::Four);
    assert_eq!(estimate(&priced).expect("sold").cents, 20);
}

#[test]
fn length_multiplies_and_nothing_rounds() {
    let at = |seconds| {
        estimate(&asking(VideoModel::Fast, VideoResolution::P720, seconds))
            .expect("sold")
            .cents
    };
    assert_eq!(at(ClipSeconds::Four), 40);
    assert_eq!(at(ClipSeconds::Six), 60);
    assert_eq!(at(ClipSeconds::Eight), 80);
}

/// Every tier and size scorsese can actually ask for has a price. If this ever
/// fails, an option was added to the editor that nothing can quote.
#[test]
fn every_shot_scorsese_can_ask_for_is_priced() {
    for model in [VideoModel::Fast, VideoModel::Lite] {
        for resolution in [VideoResolution::P720, VideoResolution::P1080] {
            for seconds in ClipSeconds::ALL {
                let request = asking(model, resolution, seconds);
                assert!(
                    estimate(&request).is_ok(),
                    "{model:?} at {resolution:?} has no price"
                );
            }
        }
    }
}

/// The tiers and sizes the table carries for auditing are the ones nobody can
/// choose in the editor — so the table being longer than the editor is
/// deliberate rather than drift.
#[test]
fn the_table_is_wider_than_what_is_offered() {
    assert!(!Tier::Standard.is_offered());
    assert!(Tier::Fast.is_offered() && Tier::Lite.is_offered());
    assert!(!Quality::P4k.is_offered());
    assert!(Quality::P720.is_offered() && Quality::P1080.is_offered());
}

#[test]
fn money_prints_as_money() {
    assert_eq!(dollars(5), "$0.05");
    assert_eq!(dollars(96), "$0.96");
    assert_eq!(dollars(100), "$1.00");
    assert_eq!(dollars(1925), "$19.25");
}
