//! A still the v55 → v56 migration renames stays drawn: `scorsese migrate`
//! carries its file and its ledger lines to the new id, so the next run
//! neither draws nor bills it again, and the report counts nothing superseded.

use std::path::Path;

use scorsese_core::{GenerationState, ImageRequest, Project, ProjectPath, migrate};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, generate};
use scorsese_providers::spending::{follow_renames, so_far};

use crate::drawing::mock::Mock;
use crate::drawing::{asset_mut, sketched};

/// "a lighthouse at dusk" at 16:9 as Flash named it before v50 (`retired.rs`).
const FLASH_2K: &str = "1009eb784bd5e2ed6973d0c4d019a7850728fbc7bbfd550ef61b4d1fa4766f8e";

/// Writes `project` into `dir` as a v55 document and migrates the folder the
/// way `scorsese migrate` does; the project as it now loads.
fn migrated(dir: &Path, project: &Project) -> Project {
    let mut document = serde_json::to_value(project).expect("serialises");
    document["schema_version"] = serde_json::json!(55);
    std::fs::write(dir.join("project.json"), document.to_string()).expect("written");
    let done = migrate::folder(dir)
        .expect("migrates")
        .expect("it was behind");
    follow_renames(dir, &done.renamed).expect("the ledger follows");
    Project::load(dir).expect("the migrated folder loads")
}

#[test]
fn a_still_drawn_under_an_id_the_migration_renames_is_not_drawn_again() {
    let (dir, mut project, id) = sketched("renamed-drawn", "a lighthouse at dusk");
    asset_mut(&mut project, &id).id = scorsese_core::AssetId::new("poster one");
    generate(&mut project, &dir, &Mock::willing(), Budget::unlimited(0)).expect("drawn");

    let mut project = migrated(&dir, &project);
    assert_eq!(project.assets[0].id.as_str(), "poster-one");
    let provider = Mock::willing();
    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_eq!(provider.requests(), 0, "nothing drawn or billed: {run:?}");
    assert!(matches!(run[0].1, Outcome::Cached { .. }), "{run:?}");
    let spent = so_far(&project, &dir);
    assert_eq!(
        spent.superseded.takes, 0,
        "the take shown is no superseded one"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_still_a_retired_model_drew_under_a_renamed_id_is_not_drawn_again() {
    let (dir, mut project, id) = sketched("renamed-retired", "a lighthouse at dusk");
    let path = ProjectPath::new(format!("generated/poster x-{FLASH_2K}.jpg"));
    let file = path.resolve(&dir).expect("a project path");
    std::fs::create_dir_all(file.parent().expect("generated/")).expect("generated/");
    std::fs::write(&file, b"a picture paid for").expect("the picture");
    let asset = asset_mut(&mut project, &id);
    asset.id = scorsese_core::AssetId::new("poster x");
    asset.state = Some(GenerationState::Generated);
    asset.path = Some(path);
    asset.image = Some(ImageRequest::default());

    let mut project = migrated(&dir, &project);
    let carried = ProjectPath::new(format!("generated/poster-x-{FLASH_2K}.jpg"));
    assert_eq!(project.assets[0].path.as_ref(), Some(&carried));
    let provider = Mock::willing();
    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_eq!(provider.requests(), 0, "nothing drawn or billed: {run:?}");
    assert_eq!(run[0].1, Outcome::Cached { path: carried });
    std::fs::remove_dir_all(&dir).ok();
}
