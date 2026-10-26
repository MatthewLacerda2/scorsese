//! What a still is estimated to cost: the picture's fixed price, and the input.

use scorsese_core::{ImageRequest, ImageResolution};
use scorsese_providers::prices::{gemini, image};

fn at(resolution: ImageResolution) -> ImageRequest {
    ImageRequest {
        resolution: Some(resolution),
        ..ImageRequest::default()
    }
}

/// Every size a still can ask for has a rate, and every row is one somebody
/// can ask for: the editor and Google's price list are the same list (#891).
#[test]
fn every_still_scorsese_can_ask_for_is_priced_and_nothing_else_is() {
    for resolution in ImageResolution::ALL {
        assert!(gemini::rate(resolution).is_some(), "{resolution:?}");
    }
    assert_eq!(ImageResolution::ALL.len(), gemini::RATES.len());
}

/// Every figure on Google's page, read 2026-10-15, written out rather than
/// looped over the table, for the Veo table test's reason: expectations derived
/// from the table prove only that it agrees with itself.
#[test]
fn the_table_says_what_the_vendors_page_says() {
    let rate = |resolution| {
        let rate = gemini::rate(resolution).expect("sold");
        (rate.microdollars_per_image, rate.cents_per_million_input)
    };
    use ImageResolution::{K1, K2, K4};
    assert_eq!(rate(K1), (33_600, 150));
    assert_eq!(rate(K2), (50_400, 150));
    assert_eq!(rate(K4), (113_000, 150));
}

/// A typical still — the default 2K, a 400-character prompt and two
/// references — is six cents.
#[test]
fn a_typical_still_costs_six_cents() {
    let typical = image(&ImageRequest::default(), 400, 2).expect("priced");
    assert_eq!(typical.cents, 6);
}

#[test]
fn the_picture_is_the_published_price_rounded_up_to_the_cent() {
    let cents = |request: &ImageRequest| image(request, 0, 0).expect("priced").cents;
    assert_eq!(cents(&at(ImageResolution::K1)), 4, "$0.0336");
    assert_eq!(cents(&ImageRequest::default()), 6, "2K, $0.0504");
    assert_eq!(cents(&at(ImageResolution::K4)), 12, "$0.113");
}

#[test]
fn references_are_counted_as_input_and_say_so() {
    // 1K is 3.36¢; fourteen references are 15,680 tokens at $1.50 a million —
    // 2.352¢ — and a 400-character prompt 100 more tokens.
    let priced = image(&at(ImageResolution::K1), 400, 14).expect("priced");
    assert_eq!(priced.input_tokens, 100 + 14 * 1_120);
    assert_eq!(priced.cents, 6);
    // 4K is 11.3¢, and the same input 2.367¢ more.
    let priced = image(&at(ImageResolution::K4), 400, 14).expect("priced");
    assert_eq!(priced.cents, 14);
}

/// The batch tier is half of every standard row, picture and input alike, and
/// rounded up to the cent once, like the standard estimate (#894).
#[test]
fn a_batch_still_is_half_the_standard_price_rounded_up() {
    assert_eq!(gemini::BATCH_PERCENT, 50);
    let small = at(ImageResolution::K1);
    let now = image(&small, 0, 0).expect("sold").cents;
    let later = scorsese_providers::prices::image_in_batch(&small, 0, 0)
        .expect("sold")
        .cents;
    assert_eq!((now, later), (4, 2), "3.36¢ now and 1.68¢ in a batch");
}
