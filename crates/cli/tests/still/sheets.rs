//! `--sheet`: several instants written as one contact sheet (#814).

use crate::common::holds;
use crate::{still, titled};

/// The width and height a PNG's header declares.
fn png_size(png: &[u8]) -> (u32, u32) {
    let word = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    (word(16), word(20))
}

/// Several instants, one file: exactly the one named, as wide as its cells.
#[test]
fn a_sheet_writes_exactly_the_file_that_was_named() {
    let dir = titled("still-sheet");
    still(&dir, "sheet.png", &["--at", "0s,1.0s,90", "--sheet"])
        .ok()
        .says("sheet.png");

    let png = std::fs::read(dir.join("sheet.png")).expect("the sheet was written");
    assert_eq!(png_size(&png), (3 * 64, 36), "three 64x36 cells in a row");
    assert!(
        !holds(&dir, "sheet-00000.png"),
        "a sheet is one file, not one per instant"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// More than a sheet holds is refused, saying the cap, and writes nothing.
#[test]
fn a_sheet_of_more_than_five_instants_is_refused() {
    let dir = titled("still-sheet-many");
    let run = still(&dir, "sheet.png", &["--at", "0,1,2,3,4,5", "--sheet"]);
    assert!(run.failed, "six cells must be refused:\n{}", run.output);
    run.says("at most 5");
    assert!(!holds(&dir, "sheet.png"), "a refused sheet left a file");
    std::fs::remove_dir_all(dir).ok();
}
