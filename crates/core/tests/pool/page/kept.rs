//! What the pool does with a page once it is in: never probed, healthy without
//! a `media` block, and collected without losing the file.

use super::{PAGE, no_probe};
use crate::common::{new_project, source_file};
use scorsese_core::pool::{remove_assets, unused_assets};
use scorsese_core::{
    AssetHealth, HashCheck, Reprobe, asset_status, import_asset, probe_assets, unprobed_assets,
};

#[test]
fn a_page_is_never_probed_and_never_reported_unprobed() {
    let (dir, mut project) = new_project("page-probe");
    let source = source_file(&dir, "title.html", PAGE);
    import_asset(&mut project, &dir, &source, None, &no_probe()).expect("imports");

    assert!(unprobed_assets(&project).is_empty());
    assert!(probe_assets(&mut project, &dir, &no_probe(), Reprobe::All).is_empty());

    let rows = asset_status(&project, &dir, HashCheck::Verify);
    assert_eq!(rows[0].health, AssetHealth::Ok);
}

#[test]
fn a_missing_page_is_reported_missing() {
    let (dir, mut project) = new_project("page-missing");
    let source = source_file(&dir, "title.html", PAGE);
    import_asset(&mut project, &dir, &source, None, &no_probe()).expect("imports");
    std::fs::remove_file(dir.join("pages/title.html")).expect("delete");

    let rows = asset_status(&project, &dir, HashCheck::Skip);
    assert_eq!(rows[0].health, AssetHealth::Missing);
    assert!(rows[0].health.needs_attention());
}

/// A page is authored: collecting its row is fine, deleting the document
/// somebody wrote is not.
#[test]
fn collecting_a_page_keeps_its_file() {
    let (dir, mut project) = new_project("page-gc");
    let source = source_file(&dir, "title.html", PAGE);
    let id = import_asset(&mut project, &dir, &source, None, &no_probe()).expect("imports");
    assert_eq!(unused_assets(&project), vec![id.clone()]);

    let report = remove_assets(&mut project, &dir, &[id]).expect("collects");

    assert!(project.assets.is_empty());
    assert_eq!(report.files_deleted, 0);
    assert!(dir.join("pages/title.html").is_file(), "the page survives");
}
