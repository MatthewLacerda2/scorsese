//! `scorsese page` — the `page_write` and `page_read` tools from a shell.

use scorsese_core::AssetKind;

use crate::common::{new_project, reload, run_in};

#[test]
fn a_page_is_written_from_a_file_rewritten_and_printed() {
    let dir = new_project("page-cli");
    let source = dir.join("first.html");
    std::fs::write(&source, "<p>first</p>").expect("a source page");

    run_in(&dir, &["page", "title", source.to_str().expect("utf-8")])
        .ok()
        .says("pages/title.html");
    let project = reload(&dir);
    assert_eq!(project.assets.len(), 1);
    assert_eq!(project.assets[0].kind, AssetKind::Html);

    std::fs::write(&source, "<p>second</p>").expect("an edit");
    run_in(&dir, &["page", "title", source.to_str().expect("utf-8")])
        .ok()
        .says("rewritten");
    assert_eq!(reload(&dir).assets.len(), 1, "the same page, not a second");

    let printed = run_in(&dir, &["page", "title"]).ok();
    assert_eq!(printed.output, "<p>second</p>");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn printing_a_page_that_is_not_there_says_so() {
    let dir = new_project("page-cli-none");
    let run = run_in(&dir, &["page", "nothing"]);
    assert!(run.failed);
    run.says("there is no asset `nothing`");
    std::fs::remove_dir_all(&dir).ok();
}
