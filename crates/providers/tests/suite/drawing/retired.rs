//! A still drawn by a model scorsese no longer offers stays drawn (#1308):
//! its file is recognised under the fingerprint that model gave it, and a run
//! neither draws nor bills it again — unless its brief has changed since.

use scorsese_core::{
    AssetId, GenerationState, ImageRequest, ImageResolution, Project, ProjectPath,
};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, Plan, generate, plan};

use crate::drawing::mock::Mock;
use crate::drawing::{asset_mut, sketched};

/// Each still's file as a retired model named it, for "a lighthouse at dusk"
/// at 16:9 — the digests `main` wrote before v50, computed from the
/// fingerprint's text.
const FLASH_2K: &str = "1009eb784bd5e2ed6973d0c4d019a7850728fbc7bbfd550ef61b4d1fa4766f8e";
const FLASH_HALF_K: &str = "c479a541cd4bb84995030e1141f37747c9c937275bd8f25a536b1a9c449fefbd";
const LITE_1K: &str = "961c11a2ec4356e18e328a438a25df237c35476cc530f731cc7d66568697fa09";
const PRO_4K: &str = "21bbf2a46479438a84479c587d8dc2c198ceb66b88487077a83aa7ce0aee3225";

/// The still `poster`, `generated` under `digest` with its file on disk, as
/// the v49 → v50 migration leaves it, asking for `resolution`.
fn drawn(
    label: &str,
    digest: &str,
    resolution: Option<ImageResolution>,
) -> (std::path::PathBuf, Project, AssetId) {
    let (dir, mut project, id) = sketched(label, "a lighthouse at dusk");
    let path = ProjectPath::new(format!("generated/poster-{digest}.jpg"));
    let file = path.resolve(&dir).expect("a project path");
    std::fs::create_dir_all(file.parent().expect("generated/")).expect("generated/");
    std::fs::write(&file, b"a picture paid for").expect("the picture");
    let asset = asset_mut(&mut project, &id);
    asset.state = Some(GenerationState::Generated);
    asset.path = Some(path);
    asset.image = Some(ImageRequest {
        resolution,
        ..ImageRequest::default()
    });
    (dir, project, id)
}

#[test]
fn a_still_each_retired_model_drew_is_not_drawn_again() {
    for (digest, resolution) in [
        (FLASH_2K, None),
        (FLASH_HALF_K, Some(ImageResolution::K1)),
        (LITE_1K, Some(ImageResolution::K1)),
        (PRO_4K, Some(ImageResolution::K4)),
    ] {
        let (dir, mut project, id) = drawn("retired", digest, resolution);
        let path = ProjectPath::new(format!("generated/poster-{digest}.jpg"));
        let asset = project.asset(&id).expect("the still");
        assert_eq!(plan(&project, &dir, asset), Plan::Realized(path.clone()));

        let provider = Mock::willing();
        let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
        assert_eq!(run[0].1, Outcome::Cached { path: path.clone() }, "{digest}");
        assert_eq!(provider.requests(), 0, "nothing is drawn or billed");
        let asset = project.asset(&id).expect("the still");
        assert_eq!(asset.path.as_ref(), Some(&path), "it plays the same file");
        std::fs::remove_dir_all(&dir).ok();
    }
}

/// The document's `generated` alone is not trusted: a still whose brief no
/// longer matches the file — here its prompt — is drawn, by 2.1.
#[test]
fn a_retired_still_whose_brief_changed_is_drawn_again() {
    let (dir, mut project, id) = drawn("changed", FLASH_2K, None);
    asset_mut(&mut project, &id).prompt = Some(String::from("a lighthouse at dawn"));
    let asset = project.asset(&id).expect("the still");
    assert_eq!(plan(&project, &dir, asset), Plan::Submit);

    let provider = Mock::willing();
    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert!(
        matches!(run[0].1, Outcome::Generated { .. }),
        "{:?}",
        run[0].1
    );
    assert_eq!(provider.requests(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

/// A retired fingerprint whose file is gone is not an answer: the still is
/// drawn, and lands under its own 2.1 name.
#[test]
fn a_retired_still_whose_file_is_gone_is_drawn_under_its_own_name() {
    let (dir, mut project, _) = drawn("gone", FLASH_2K, None);
    std::fs::remove_file(dir.join(format!("generated/poster-{FLASH_2K}.jpg"))).expect("removed");
    let provider = Mock::willing();
    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let Outcome::Generated { path, .. } = &run[0].1 else {
        panic!("{:?}", run[0].1);
    };
    assert!(!path.as_str().contains(FLASH_2K), "{path}");
    std::fs::remove_dir_all(&dir).ok();
}
