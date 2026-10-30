//! Reading a page back: its document as written, or a refusal that says
//! which of the four things was wrong.

use crate::common::new_project;
use scorsese_core::{AssetId, PageReadError, read_page, write_page};

#[test]
fn a_written_page_reads_back_exactly() {
    let (dir, mut project) = new_project("page-read-back");
    write_page(&mut project, &dir, "title", "<p>Hi</p>").expect("written");

    let html = read_page(&project, &dir, &AssetId::new("title")).expect("reads");

    assert_eq!(html, "<p>Hi</p>");
}

#[test]
fn a_name_nothing_answers_to_is_no_asset_and_names_no_surface() {
    let (dir, project) = new_project("page-read-none");

    let refused = read_page(&project, &dir, &AssetId::new("nope")).expect_err("refused");

    assert!(
        matches!(refused, PageReadError::NoAsset { .. }),
        "{refused:?}"
    );
    assert_eq!(refused.to_string(), "there is no asset `nope`");
}

#[test]
fn a_page_whose_file_is_gone_is_unreadable() {
    let (dir, mut project) = new_project("page-read-gone");
    let written = write_page(&mut project, &dir, "title", "<p>Hi</p>").expect("written");
    std::fs::remove_file(written.path.resolve(&dir).expect("a project path")).expect("removed");

    let refused = read_page(&project, &dir, &written.id).expect_err("refused");

    assert!(
        matches!(refused, PageReadError::Unreadable { .. }),
        "{refused:?}"
    );
    assert!(
        refused
            .to_string()
            .starts_with("the page `title` at pages/title.html could not be read: "),
        "{refused}"
    );
}
