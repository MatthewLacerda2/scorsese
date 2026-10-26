//! What a generation quote charges, and what moves its digest.

use scorsese_core::AssetId;
use scorsese_providers::quote::generation;
use scorsese_providers::video::Brief;

use super::sketched;
use crate::common::write;

#[test]
fn a_sketched_shot_and_line_are_both_charged() {
    let (dir, project) = sketched("quote-both");
    let quote = generation(&project, &dir).expect("quote");

    assert!(!quote.is_free());
    let charged: Vec<&str> = quote
        .items
        .iter()
        .filter(|item| item.charge.is_some())
        .map(|item| item.subject.as_str())
        .collect();
    assert_eq!(charged, ["shot", "vo"]);
    // Eight seconds at the default Fast 1080p is 96¢; six characters is 1¢.
    assert_eq!(quote.cents(), 97);
    std::fs::remove_dir_all(dir).ok();
}

/// The digest is what a token is bound to, so editing a brief must move it.
#[test]
fn editing_a_brief_moves_the_digest() {
    let (dir, mut project) = sketched("quote-edited");
    let before = generation(&project, &dir).expect("quote").digest();

    let shot = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == AssetId::new("shot"))
        .expect("the shot");
    shot.prompt = Some(String::from("a boat at dusk"));

    assert_ne!(generation(&project, &dir).expect("quote").digest(), before);
    std::fs::remove_dir_all(dir).ok();
}

/// A brief whose output is already in `generated/` is charged nothing,
/// because the run would send nothing — and is marked made, so no surface
/// shows it beside what costs money (#1436).
#[test]
fn a_brief_already_on_disk_is_made_and_not_charged() {
    let (dir, project) = sketched("quote-cached");
    let shot = project.asset(&AssetId::new("shot")).expect("the shot");
    let output = Brief::of(&project, &dir, shot).expect("brief").output();
    write(&dir, output.as_str(), "the take");

    let quote = generation(&project, &dir).expect("quote");
    let item = &quote.items[0];
    assert!(item.charge.is_none(), "{item:?}");
    assert!(item.says.contains("already generated"), "{}", item.says);
    assert!(item.made, "{item:?}");
    assert_eq!(quote.cents(), 1, "only the line is left to pay for");
    let shown: Vec<&str> = quote.shown().map(|item| item.subject.as_str()).collect();
    assert_eq!((shown, quote.made()), (vec!["vo"], 1));
    std::fs::remove_dir_all(dir).ok();
}

/// Reordering the table changes nothing anybody would pay for.
#[test]
fn the_digest_ignores_document_order() {
    let (dir, mut project) = sketched("quote-order");
    let before = generation(&project, &dir).expect("quote").digest();
    project.assets.reverse();
    assert_eq!(generation(&project, &dir).expect("quote").digest(), before);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_project_with_nothing_prompted_is_free() {
    let (dir, project) = crate::common::project("quote-nothing");
    assert!(generation(&project, &dir).expect("quote").is_free());
    std::fs::remove_dir_all(dir).ok();
}

/// A shot's prompt past what Veo reads is never quoted or sent (#1309).
#[test]
fn a_shot_prompt_past_veos_limit_would_not_be_sent() {
    let (dir, mut project) = sketched("quote-long");
    let shot = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == AssetId::new("shot"))
        .expect("the shot");
    shot.prompt = Some("a".repeat(scorsese_core::MAX_PROMPT_CHARACTERS + 1));

    let quote = generation(&project, &dir).expect("quote");
    let item = &quote.items[0];
    assert!(item.charge.is_none(), "{item:?}");
    assert!(
        item.says.contains("Veo reads at most 1024 tokens"),
        "{}",
        item.says
    );
    assert!(!item.made, "why it is held back is still shown");
    std::fs::remove_dir_all(dir).ok();
}
