//! A design is a line in `generations.json`, written when it is paid for
//! (#1486), so neither its record in `generated/` being overwritten nor that
//! folder being deleted takes it off the ceiling.

use scorsese_core::AssetId;
use scorsese_providers::credentials::Budget;
use scorsese_providers::spending::{GENERATIONS_FILE, history, so_far};
use scorsese_providers::voices::design::{design, spent};

use super::fake::Fake;
use super::{brief, root};
use crate::common::{project, synth_asset};

/// Its samples deleted, the same design is paid for again — and its record is
/// rewritten in place, so only the ledger still says it was paid for twice.
#[test]
fn a_design_paid_twice_for_one_digest_counts_twice() {
    let dir = root("design-twice");
    let studio = Fake::offering(&["a", "b", "c"]);
    let brief = brief("a narrator", Some(1));
    let first = design(&dir, &studio, &brief, Budget::unlimited(0)).expect("the first design");
    for sample in &first.session.candidates {
        std::fs::remove_file(sample.path.resolve(&dir).expect("a project path"))
            .expect("removing a sample");
    }

    let again = design(&dir, &studio, &brief, Budget::unlimited(0)).expect("the design again");

    assert!(!again.already_paid);
    assert_eq!(studio.designs(), 2);
    assert_eq!(spent(&dir), 2 * first.estimate.cents);
}

#[test]
fn a_design_whose_generated_folder_is_gone_still_counts() {
    let dir = root("design-no-generated");
    let studio = Fake::offering(&["a", "b", "c"]);
    let designing = design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect("the design");

    std::fs::remove_dir_all(dir.join("generated")).expect("removing generated/");

    assert_eq!(spent(&dir), designing.estimate.cents);
}

/// A design paid for before design lines began is read off its record, and
/// written down as a seeded line when the next design is paid for.
#[test]
fn a_design_older_than_the_ledger_is_seeded_from_its_record() {
    let dir = root("design-seeded");
    let studio = Fake::offering(&["a", "b", "c"]);
    let older = design(
        &dir,
        &studio,
        &brief("a narrator", Some(1)),
        Budget::unlimited(0),
    )
    .expect("the older design");
    std::fs::remove_file(dir.join(GENERATIONS_FILE)).expect("as if before the ledger");
    assert_eq!(spent(&dir), older.estimate.cents, "read off its record");

    let newer = design(
        &dir,
        &studio,
        &brief("a narrator", Some(2)),
        Budget::unlimited(0),
    )
    .expect("the newer design");
    std::fs::remove_dir_all(dir.join("generated")).expect("removing generated/");

    assert_eq!(spent(&dir), older.estimate.cents + newer.estimate.cents);
    let file = std::fs::read_to_string(dir.join(GENERATIONS_FILE)).expect("the ledger");
    assert_eq!(file.matches("\"seeded\": true").count(), 1, "{file}");
}

/// A design line is not a generation: a project whose ledger holds only design
/// lines still reads its assets' figures as the generations it began with.
#[test]
fn a_design_line_leaves_the_generations_seeded_from_the_assets() {
    let (dir, mut project) = project("design-and-assets");
    let id = synth_asset(&mut project, "shot", "recipes/shot.json");
    project
        .asset_mut(&id)
        .expect("the asset")
        .estimated_cost_cents = Some(40);
    let studio = Fake::offering(&["a", "b", "c"]);
    let designing = design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect("the design");

    let lines = history(&project, &dir);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].seeded && lines[0].asset == AssetId::new("shot"));
    let spending = so_far(&project, &dir);
    assert_eq!(
        (spending.generation_cents, spending.design_cents),
        (40, designing.estimate.cents)
    );
}

/// A ledger that does not read is refused before the vendor is asked, so the
/// design is never paid for without its line.
#[test]
fn a_damaged_ledger_is_refused_before_paying() {
    let dir = root("design-damaged");
    std::fs::write(dir.join(GENERATIONS_FILE), "{ not a ledger").expect("write");
    let studio = Fake::offering(&["a", "b", "c"]);

    design(
        &dir,
        &studio,
        &brief("a narrator", None),
        Budget::unlimited(0),
    )
    .expect_err("a damaged ledger is refused");

    assert_eq!(studio.designs(), 0, "the vendor was asked anyway");
}
