//! `scorsese migrate` — carrying a folder forward, and refusing what it cannot.
//!
//! There is no step yet (v33 is the oldest this build migrates from), so what
//! can be asserted today is the edges: a current folder is not touched, and a
//! document outside the range is refused and left exactly as it was. The chain
//! itself is tested in `scorsese_core::migrate`, over a chain that has steps.

use scorsese_core::{PROJECT_FILE_NAME, SCHEMA_VERSION};

use crate::common::{new_project, run_in};

#[test]
fn a_current_project_is_left_byte_for_byte_alone() {
    let dir = new_project("migrate-current");
    let file = dir.join(PROJECT_FILE_NAME);
    let before = std::fs::read(&file).unwrap();

    run_in(&dir, &["migrate"]).ok().says("nothing to do");

    assert_eq!(std::fs::read(&file).unwrap(), before);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_document_outside_the_range_is_refused_and_untouched() {
    let dir = new_project("migrate-refused");
    let file = dir.join(PROJECT_FILE_NAME);
    let current = std::fs::read_to_string(&file).unwrap();
    for version in [SCHEMA_VERSION + 1, 0] {
        let document = current.replacen(
            &format!("\"schema_version\": {SCHEMA_VERSION}"),
            &format!("\"schema_version\": {version}"),
            1,
        );
        assert_ne!(document, current, "the version was rewritten");
        std::fs::write(&file, &document).unwrap();

        let run = run_in(&dir, &["migrate"]);
        assert!(run.failed, "{version}: {}", run.output);
        run.says(&format!("schema_version {version}"));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), document);
    }
    std::fs::remove_dir_all(&dir).ok();
}
