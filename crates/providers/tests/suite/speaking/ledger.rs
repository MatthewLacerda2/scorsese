//! A narration line spoken leaves its line in the ledger (#1059), and an
//! unreadable ledger stops the run before anything is spoken.

use scorsese_core::{AssetId, SpeechModel};
use scorsese_providers::credentials::Budget;
use scorsese_providers::speech::{generate, quote};
use scorsese_providers::spending::{GENERATIONS_FILE, history};

use crate::speaking::mock::{Answer, Mock};
use crate::speaking::sketched;

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

/// A completed call that spoke nothing usable is billed (#1695): its take is
/// in the ledger, it counts against the run's ceiling, and the run says what
/// it cost.
#[test]
fn a_billed_call_that_spoke_nothing_is_charged_and_counts_against_the_ceiling() {
    let line = "Night City never sleeps.";
    let (dir, mut project, id) = sketched("ledger-speak-billed", line);
    let mut second = project.asset(&id).expect("the line").clone();
    second.id = AssetId::new("vo-2");
    project.assets.push(second);
    let cents = quote(SpeechModel::default(), line.chars().count());
    assert!(cents > 0, "a line costs something");
    let provider = Mock::giving(vec![Answer::Bills(String::from("not audio"))]);

    let stopped = generate(&mut project, &dir, &provider, Budget::new(2 * cents - 1, 0))
        .expect_err("the billed take leaves no room for a second");

    assert_eq!(provider.requests(), 1, "the second was never sent");
    let lines = history(&project, &dir);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].asset, id);
    assert_eq!(lines[0].estimated_cost_cents, cents);
    assert!(!lines[0].refused, "charged, not released");
    let (_, outcome) = &stopped.run[0];
    assert_eq!(outcome.spent_cents(), cents, "{outcome:?}");
    assert!(outcome.says().contains('$'), "{}", outcome.says());
    let says = stopped.says();
    assert!(
        says.contains("0 lines were spoken"),
        "nothing is kept: {says}"
    );
    assert!(
        says.contains("1 line was refused and billed anyway (vo)"),
        "{says}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A refusal the vendor does not bill leaves no line and spends nothing.
#[test]
fn an_unbilled_refusal_is_not_charged() {
    let (dir, mut project, _) = sketched("ledger-speak-unbilled", "Night City never sleeps.");
    let provider = Mock::giving(vec![Answer::Refuses(String::from("no"))]);
    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("refused");

    assert!(history(&project, &dir).is_empty());
    assert_eq!(run[0].1.spent_cents(), 0);
    assert!(!run[0].1.says().contains('$'), "{}", run[0].1.says());
    std::fs::remove_dir_all(&dir).ok();
}
