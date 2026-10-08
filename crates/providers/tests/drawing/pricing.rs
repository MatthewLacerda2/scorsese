//! What a still is estimated to cost: the picture's fixed price, and the input.

use scorsese_core::{ImageModel, ImageRequest, ImageResolution};
use scorsese_providers::prices::{gemini, image};

fn at(model: ImageModel, resolution: ImageResolution) -> ImageRequest {
    ImageRequest {
        model,
        resolution: Some(resolution),
        ..ImageRequest::default()
    }
}

/// Every size a model draws has a rate, and every row is one somebody can ask
/// for: the editor and Google's price list are the same list (#891).
#[test]
fn every_still_scorsese_can_ask_for_is_priced_and_nothing_else_is() {
    let mut asked = 0;
    for model in ImageModel::ALL {
        for resolution in ImageResolution::ALL {
            assert_eq!(
                gemini::rate(model, resolution).is_some(),
                model.supports(resolution),
                "{model:?} at {resolution:?}"
            );
            asked += usize::from(model.supports(resolution));
        }
    }
    assert_eq!(asked, gemini::RATES.len());
}

/// Every figure on Google's page, read 2026-10-08, written out rather than
/// looped over the table, for the Veo table test's reason: expectations derived
/// from the table prove only that it agrees with itself.
#[test]
fn the_table_says_what_the_vendors_page_says() {
    let rate = |model, resolution| {
        let rate = gemini::rate(model, resolution).expect("sold");
        (rate.microdollars_per_image, rate.cents_per_million_input)
    };
    use ImageModel::{Flash, Lite, NanoBanana21, Pro};
    use ImageResolution::{K1, K2, K4, K05};
    assert_eq!(rate(NanoBanana21, K1), (33_600, 150));
    assert_eq!(rate(NanoBanana21, K2), (50_400, 150));
    assert_eq!(rate(NanoBanana21, K4), (113_000, 150));
    assert_eq!(rate(Flash, K05), (45_000, 50));
    assert_eq!(rate(Flash, K1), (67_000, 50));
    assert_eq!(rate(Flash, K2), (101_000, 50));
    assert_eq!(rate(Flash, K4), (151_000, 50));
    assert_eq!(rate(Lite, K1), (33_600, 25));
    assert_eq!(rate(Pro, K1), (134_000, 200));
    assert_eq!(rate(Pro, K2), (134_000, 200));
    assert_eq!(rate(Pro, K4), (240_000, 200));
}

/// The maintainer's test for the default (#893): 2.1 becomes it only if a
/// typical 2K still costs no more than on Flash, input counted. It costs about
/// half.
#[test]
fn the_default_still_costs_less_than_the_same_still_on_flash() {
    let typical = |model| image(&at(model, ImageResolution::K2), 400, 2).expect("priced");
    assert_eq!(ImageRequest::default().model, ImageModel::NanoBanana21);
    assert_eq!(typical(ImageModel::NanoBanana21).cents, 6);
    assert_eq!(typical(ImageModel::Flash).cents, 11);
}

#[test]
fn the_picture_is_the_published_price_rounded_up_to_the_cent() {
    let cents = |request: &ImageRequest| image(request, 0, 0).expect("priced").cents;
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K05)), 5);
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K1)), 7);
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K2)), 11);
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K4)), 16);
    assert_eq!(cents(&ImageRequest::default()), 6, "2.1 at 2K, $0.0504");
    assert_eq!(
        cents(&ImageRequest {
            model: ImageModel::Lite,
            ..ImageRequest::default()
        }),
        4
    );
}

#[test]
fn references_are_counted_as_input_and_say_so() {
    // 1K flash is 6.7¢; fourteen references are 15,680 tokens at $0.50 a
    // million — 0.784¢ — and a 400-character prompt 100 more tokens.
    let priced = image(&at(ImageModel::Flash, ImageResolution::K1), 400, 14).expect("priced");
    assert_eq!(priced.input_tokens, 100 + 14 * 1_120);
    assert_eq!(priced.cents, 8);
    // 4K Pro is 24¢; fourteen references at $2 a million are 3.136¢ more.
    let priced = image(&at(ImageModel::Pro, ImageResolution::K4), 400, 14).expect("priced");
    assert_eq!(priced.cents, 28);
}

#[test]
fn a_size_the_model_does_not_draw_has_no_price() {
    assert!(image(&at(ImageModel::Lite, ImageResolution::K4), 10, 0).is_err());
    assert!(image(&at(ImageModel::NanoBanana21, ImageResolution::K05), 10, 0).is_err());
}

/// The batch tier is half of every standard row, picture and input alike, and
/// rounded up to the cent once, like the standard estimate (#894).
#[test]
fn a_batch_still_is_half_the_standard_price_rounded_up() {
    assert_eq!(gemini::BATCH_PERCENT, 50);
    let flash = at(ImageModel::Flash, ImageResolution::K1);
    let now = image(&flash, 0, 0).expect("sold").cents;
    let later = scorsese_providers::prices::image_in_batch(&flash, 0, 0)
        .expect("sold")
        .cents;
    assert_eq!((now, later), (7, 4), "6.7¢ now and 3.35¢ in a batch");
}
