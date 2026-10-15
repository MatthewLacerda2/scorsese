//! Importing several paths at once: one that fails costs nothing of the
//! others, and the batch says whether the document needs saving.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{import_many, import_sequences};

#[test]
fn a_failing_path_does_not_cost_the_ones_around_it() {
    let (dir, mut project) = new_project("many-carry-on");
    let wide = source_file(&dir, "wide.mp4", b"the wide shot");
    let close = source_file(&dir, "close.mp4", b"the close-up");
    let missing = dir.join("sources/missing.mp4");

    let paths = [wide, missing.clone(), close];
    let batch = import_many(&mut project, &dir, &paths, None, &StubProbe::video());

    let ids: Vec<_> = batch
        .reports
        .iter()
        .flat_map(|report| &report.imported)
        .map(|one| one.id.as_str())
        .collect();
    assert_eq!(ids, ["wide", "close"], "in the order given");
    assert_eq!(batch.failures.len(), 1);
    assert_eq!(batch.failures[0].0, missing);
    assert!(batch.needs_save);
}

#[test]
fn media_already_in_the_pool_needs_no_save() {
    let (dir, mut project) = new_project("many-again");
    let wide = source_file(&dir, "wide.mp4", b"the wide shot");
    let probe = StubProbe::video();
    import_many(&mut project, &dir, &[&wide], None, &probe);

    let again = import_many(&mut project, &dir, &[&wide], None, &probe);

    assert!(again.failures.is_empty());
    assert!(!again.needs_save, "the same bytes add nothing");
}

#[test]
fn a_folder_that_is_not_there_fails_alone_and_needs_no_save() {
    let (dir, mut project) = new_project("many-sequences");

    let batch = import_sequences(
        &mut project,
        &dir,
        &[dir.join("no-frames-here")],
        &StubProbe::image(),
    );

    assert!(batch.reports.is_empty());
    assert_eq!(batch.failures.len(), 1);
    assert!(!batch.needs_save);
}
