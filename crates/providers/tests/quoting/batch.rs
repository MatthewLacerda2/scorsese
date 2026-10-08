//! A batch's quote (#894): stills alone, at half the rate, under a spend of
//! its own — and the offer a quote for now makes beside itself.

use scorsese_core::{
    Asset, AssetId, AssetKind, GenerationState, ImageModel, ImageRequest, ImageResolution,
};
use scorsese_providers::quote::{OFFER_FROM_CENTS, Spend, Unquotable, batch, generation, offer};

use super::sketched;
use crate::common::project;

/// A project of `count` sketched stills at Pro's 4K: 24¢ a picture and a few
/// micro-dollars of prompt, so 25¢ each now and 13¢ in a batch — each rounded
/// up to the cent on its own.
fn stills(label: &str, count: usize) -> (std::path::PathBuf, scorsese_core::Project) {
    let (dir, mut project) = project(label);
    for n in 0..count {
        let mut still = Asset::sketch(
            AssetId::new(format!("still-{n}")),
            AssetKind::GeneratedImage,
            format!("frame {n}"),
        );
        still.image = Some(ImageRequest {
            model: ImageModel::Pro,
            resolution: Some(ImageResolution::K4),
            ..ImageRequest::default()
        });
        project.assets.push(still);
    }
    (dir, project)
}

#[test]
fn a_batch_is_half_the_price_and_its_own_kind_of_spending() {
    let (dir, project) = stills("quote-batch-half", 2);
    let now = generation(&project, &dir).expect("quote");
    let later = batch(&project, &dir).expect("quote");

    assert_eq!(now.cents(), 50);
    assert_eq!(later.cents(), 26);
    assert_eq!(later.spend, Spend::Batch);
    assert_ne!(
        later.digest(),
        now.digest(),
        "a token for one is not one for the other"
    );
    assert!(
        later.items[0].says.contains("within 24 hours"),
        "{}",
        later.items[0].says
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_batch_with_a_shot_or_a_line_to_send_is_refused_naming_them() {
    let (dir, project) = sketched("quote-batch-refused");
    let refused = batch(&project, &dir).expect_err("only stills batch");
    assert_eq!(
        refused,
        Unquotable::Unbatchable(vec![String::from("shot"), String::from("vo")])
    );
    assert!(refused.to_string().contains("shot, vo"), "{refused}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_still_waiting_in_its_batch_is_quoted_at_nothing() {
    let (dir, mut project) = stills("quote-batch-waiting", 1);
    project.assets[0].state = Some(GenerationState::Queued);
    project.assets[0].operation = Some(String::from("batches/1"));

    let quote = generation(&project, &dir).expect("quote");
    assert!(quote.is_free());
    assert!(quote.items[0].says.contains("waiting in its batch"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_batch_is_offered_only_when_the_stills_are_worth_a_dollar() {
    let (dir, project) = stills("quote-offer-small", 3);
    assert_eq!(
        offer(&project, &dir).expect("priced"),
        None,
        "75¢ is pennies"
    );
    std::fs::remove_dir_all(dir).ok();

    let (dir, project) = stills("quote-offer-worth", 4);
    let offered = offer(&project, &dir)
        .expect("priced")
        .expect("$1 is offered");
    assert_eq!(offered.now_cents, OFFER_FROM_CENTS);
    assert_eq!(offered.batch_cents, 52);
    assert!(
        offered.says().contains("$1.00 now · $0.52"),
        "{}",
        offered.says()
    );
    std::fs::remove_dir_all(dir).ok();
}
