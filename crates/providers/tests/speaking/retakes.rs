//! A line spoken again after its words changed: the old reading's
//! measurements must go with it (#1105).
//!
//! Every local caller probes with `Reprobe::Skip`, which skips an asset that
//! already carries `media`. A block left over from the shorter reading would
//! be kept for ever, and its clips fitted to a length the file no longer has.

use scorsese_core::{MediaMetadata, Project};
use scorsese_providers::credentials::Budget;
use scorsese_providers::speech::generate;

use crate::mock::Mock;
use crate::{asset_mut, sketched};

/// What a probe of the first reading would have written.
fn measured(project: &mut Project, id: &scorsese_core::AssetId) {
    asset_mut(project, id).media = Some(MediaMetadata {
        duration_seconds: Some(1.5),
        ..MediaMetadata::default()
    });
}

#[test]
fn a_line_spoken_again_longer_is_measured_again() {
    let (dir, mut project, id) = sketched("retake", "short");
    let provider = Mock::willing();
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    measured(&mut project, &id);
    asset_mut(&mut project, &id).estimated_cost_cents = Some(999);
    let first = project.asset(&id).and_then(|asset| asset.path.clone());

    asset_mut(&mut project, &id).prompt = Some(String::from("a much longer line than before"));
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a retake");

    let asset = project.asset(&id).expect("the asset");
    assert_ne!(asset.path, first, "the new reading is a new file");
    assert!(
        asset.media.is_none(),
        "the old reading's length must not survive, or the probe skips it"
    );
    assert_ne!(
        asset.estimated_cost_cents,
        Some(999),
        "nor its figure: a new file is priced as itself"
    );
    assert_eq!(provider.requests(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

/// Finding the same file again is not a retake: what was measured off it, and
/// the figure it was recorded at, both stand.
#[test]
fn the_same_reading_found_again_keeps_its_measurements() {
    let (dir, mut project, id) = sketched("refound", "the same words");
    let provider = Mock::willing();
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    measured(&mut project, &id);
    asset_mut(&mut project, &id).estimated_cost_cents = Some(1);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a cached run");

    let asset = project.asset(&id).expect("the asset");
    assert!(asset.media.is_some(), "the same file, the same measurement");
    assert_eq!(asset.estimated_cost_cents, Some(1), "and the same figure");
    assert_eq!(provider.requests(), 1, "nothing was spoken twice");
    std::fs::remove_dir_all(&dir).ok();
}
