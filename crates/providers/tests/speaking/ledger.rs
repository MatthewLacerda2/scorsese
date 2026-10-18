//! A narration line spoken leaves its line in the ledger (#1059), and an
//! unreadable ledger stops the run before anything is spoken.

use scorsese_providers::credentials::Budget;
use scorsese_providers::speech::generate;
use scorsese_providers::spending::{GENERATIONS_FILE, history};

use crate::mock::Mock;
use crate::sketched;

#[test]
fn a_line_spoken_writes_its_ledger_line() {
    let (dir, mut project, id) = sketched("ledger-speak", "Night City never sleeps.");
    let run = generate(&mut project, &dir, &Mock::willing(), Budget::unlimited(0)).expect("spoken");

    let lines = history(&project, &dir);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        !lines[0].seeded,
        "written when sent, not read off the asset"
    );
    assert_eq!(lines[0].asset, id);
    assert_eq!(lines[0].estimated_cost_cents, run[0].1.spent_cents());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unreadable_ledger_stops_the_run_before_it_speaks() {
    let (dir, mut project, _) = sketched("ledger-speak-damaged", "Night City never sleeps.");
    std::fs::write(dir.join(GENERATIONS_FILE), "{ not a ledger").expect("write");
    let provider = Mock::willing();

    let stopped = generate(&mut project, &dir, &provider, Budget::unlimited(0));

    assert!(stopped.is_err());
    assert_eq!(provider.requests(), 0, "nothing paid for");
    std::fs::remove_dir_all(&dir).ok();
}
