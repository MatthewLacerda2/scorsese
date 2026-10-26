//! A track composed leaves its line in the ledger (#1059), and an unreadable
//! ledger stops the run before anything is composed.

use scorsese_providers::music;
use scorsese_providers::spending::{GENERATIONS_FILE, history};

use super::{Composer, free, holding, track};

#[test]
fn a_track_composed_writes_its_ledger_line() {
    let (dir, mut project) = holding("ledger-compose", vec![track(Some(40))]);
    let run = music::generate(&mut project, &dir, &Composer::default(), free()).expect("composed");

    let lines = history(&project, &dir);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        !lines[0].seeded,
        "written when sent, not read off the asset"
    );
    assert_eq!(lines[0].asset, project.assets[0].id);
    assert_eq!(lines[0].estimated_cost_cents, run[0].1.spent_cents());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unreadable_ledger_stops_the_run_before_it_composes() {
    let (dir, mut project) = holding("ledger-compose-damaged", vec![track(Some(40))]);
    std::fs::write(dir.join(GENERATIONS_FILE), "{ not a ledger").expect("write");
    let composer = Composer::default();

    let stopped = music::generate(&mut project, &dir, &composer, free());

    assert!(stopped.is_err());
    assert_eq!(composer.calls.get(), 0, "nothing paid for");
    std::fs::remove_dir_all(&dir).ok();
}
