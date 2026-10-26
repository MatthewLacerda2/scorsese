//! A still drawn now and a still ordered in a batch each leave their line in
//! the ledger (#1059), and a batch whose ledger fails part-way keeps every
//! ticket it was given.

use scorsese_core::{Asset, AssetId, AssetKind};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Batch, Brief, ImageProvider, ProviderError, batch, generate};
use scorsese_providers::spending::{GENERATIONS_FILE, history};

use crate::drawing::mock::Mock;
use crate::drawing::sketched;

#[test]
fn a_still_drawn_now_writes_its_line() {
    let (dir, mut project, id) = sketched("ledger-draw", "a lighthouse at dusk");
    let run = generate(&mut project, &dir, &Mock::willing(), Budget::unlimited(0)).expect("drawn");

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
fn a_still_ordered_in_a_batch_writes_its_line() {
    let (dir, mut project, id) = sketched("ledger-batch", "a lighthouse at dusk");
    let run = batch(&mut project, &dir, &Mock::willing(), Budget::unlimited(0)).expect("ordered");

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

/// Orders like [`Mock`], and damages the ledger while the job is out — as
/// another hand editing it would.
struct Damaging {
    mock: Mock,
    ledger: std::path::PathBuf,
}

impl ImageProvider for Damaging {
    fn draw(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.mock.draw(brief)
    }

    fn name(&self) -> &'static str {
        "Damaging"
    }

    fn order(&self, briefs: &[&Brief]) -> Result<String, ProviderError> {
        std::fs::write(&self.ledger, "{ not a ledger").expect("write");
        self.mock.order(briefs)
    }

    fn ask(&self, operation: &str) -> Result<Batch, ProviderError> {
        self.mock.ask(operation)
    }
}

#[test]
fn a_ledger_that_fails_part_way_leaves_every_ordered_still_its_ticket() {
    let (dir, mut project, first) = sketched("ledger-part", "a lighthouse");
    let second = AssetId::new("second");
    project.assets.push(Asset::sketch(
        second.clone(),
        AssetKind::GeneratedImage,
        "a harbour",
    ));
    let provider = Damaging {
        mock: Mock::willing(),
        ledger: dir.join(GENERATIONS_FILE),
    };

    let stopped = batch(&mut project, &dir, &provider, Budget::unlimited(0))
        .expect_err("the ledger no longer reads");

    for id in [&first, &second] {
        let ticket = project.asset(id).and_then(|asset| asset.operation.clone());
        assert_eq!(
            ticket.as_deref(),
            Some("batches/1"),
            "{id:?} keeps its ticket"
        );
    }
    assert_eq!(stopped.run.len(), 2, "{}", stopped.says());
    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("asked after");
    assert_eq!(
        provider.mock.ordered.borrow().len(),
        1,
        "nothing ordered twice"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unreadable_ledger_stops_a_still_before_it_is_drawn_or_ordered() {
    let (dir, mut project, _) = sketched("ledger-draw-damaged", "a lighthouse");
    std::fs::write(dir.join(GENERATIONS_FILE), "{ not a ledger").expect("write");
    let provider = Mock::willing();

    assert!(generate(&mut project, &dir, &provider, Budget::unlimited(0)).is_err());
    assert!(batch(&mut project, &dir, &provider, Budget::unlimited(0)).is_err());
    assert_eq!(provider.requests(), 0, "nothing drawn");
    assert!(provider.ordered.borrow().is_empty(), "nothing ordered");
    std::fs::remove_dir_all(&dir).ok();
}
