use super::*;

#[test]
fn a_ratio_is_kept_in_lowest_terms_and_spelled_as_written() {
    let tall: Ratio = "1080:1920".parse().expect("a ratio");
    assert_eq!(tall, Ratio::TALL);
    assert_eq!(tall.to_string(), "9:16");
    assert_eq!(
        serde_json::to_string(&tall).expect("serialises"),
        "\"9:16\""
    );
    assert_eq!(Ratio::from(Aspect::Tall), Ratio::TALL);
    assert_eq!(Ratio::from(Aspect::Wide), Ratio::WIDE);
    let read: Ratio = serde_json::from_str("\"4:5\"").expect("deserialises");
    assert_eq!((read.across(), read.down()), (4, 5));
}

#[test]
fn a_ribbon_a_zero_and_a_raster_are_not_ratios() {
    assert_eq!(
        "8:1".parse::<Ratio>(),
        Err(RatioError::TooFar { across: 8, down: 1 })
    );
    assert_eq!("0:9".parse::<Ratio>(), Err(RatioError::Zero));
    assert_eq!("1920x1080".parse::<Ratio>(), Err(RatioError::Malformed));
    assert!(serde_json::from_str::<Ratio>("\"wide\"").is_err());
}

#[test]
fn a_measured_size_reads_as_the_common_shape_it_is_near() {
    assert_eq!(Ratio::of_size(1920, 1088), Some(Ratio::WIDE));
    assert_eq!(Ratio::of_size(1366, 768), Some(Ratio::WIDE));
    assert_eq!(Ratio::of_size(1000, 700), Ratio::new(10, 7).ok());
    assert_eq!(Ratio::of_size(40, 1000), None);
}

#[test]
fn a_delivery_has_1080_lines_on_its_shorter_side() {
    assert_eq!(Ratio::WIDE.delivery(), (1920, 1080));
    assert_eq!(Ratio::TALL.delivery(), (1080, 1920));
    assert_eq!(Ratio::new(1, 1).expect("square").delivery(), (1080, 1080));
    assert_eq!(Ratio::new(4, 5).expect("portrait").delivery(), (1080, 1350));
    assert_eq!(Ratio::new(21, 9).expect("scope").delivery(), (2520, 1080));
}

#[test]
fn a_budget_is_spent_in_the_shape_on_even_sides() {
    assert_eq!(Ratio::WIDE.spend(1280 * 720), (1280, 720));
    assert_eq!(Ratio::TALL.spend(1280 * 720), (720, 1280));
    assert_eq!(Ratio::TALL.spend(640 * 360), (360, 640));
    let (width, height) = Ratio::new(4, 5).expect("portrait").spend(640 * 360);
    assert!(width % 2 == 0 && height % 2 == 0, "{width}x{height}");
}

#[test]
fn even_rounds_to_the_nearest_even_side_and_never_below_two() {
    assert_eq!(even(337.5), 338);
    assert_eq!(even(336.9), 336);
    assert_eq!(even(0.2), 2);
}
