//! `scorsese stock import` names at most 20 ids, as the `stock_import` tool
//! does (#1652): a longer list is refused before any library is asked, so
//! the run says so with no key and no network.

use crate::common::{new_project, run_in};

#[test]
fn an_import_of_more_than_twenty_ids_is_refused_before_any_library_is_asked() {
    let dir = new_project("stock-twenty-one");
    let ids: Vec<String> = (1..=21).map(|id| id.to_string()).collect();
    for kind in [None, Some("--image"), Some("--lottie")] {
        let mut arguments = vec!["stock", "import"];
        arguments.extend(kind);
        arguments.extend(ids.iter().map(String::as_str));
        let run = run_in(&dir, &arguments);
        assert!(run.failed, "{kind:?}: {}", run.output);
        run.says("name at most 20");
    }
    std::fs::remove_dir_all(&dir).ok();
}
