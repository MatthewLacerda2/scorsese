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

#[test]
fn every_size_flash_draws_and_the_one_lite_draws_has_a_rate() {
    for resolution in ImageResolution::ALL {
        assert!(gemini::rate(ImageModel::Flash, resolution).is_some());
        assert_eq!(
            gemini::rate(ImageModel::Lite, resolution).is_some(),
            ImageModel::Lite.supports(resolution)
        );
    }
}

#[test]
fn the_picture_is_the_published_price_rounded_up_to_the_cent() {
    let cents = |request: &ImageRequest| image(request, 0, 0).expect("priced").cents;
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K05)), 5);
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K1)), 7);
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K2)), 11);
    assert_eq!(cents(&at(ImageModel::Flash, ImageResolution::K4)), 16);
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
}

#[test]
fn a_size_the_model_does_not_draw_has_no_price() {
    assert!(image(&at(ImageModel::Lite, ImageResolution::K4), 10, 0).is_err());
}
