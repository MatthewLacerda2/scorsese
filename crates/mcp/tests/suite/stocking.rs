//! One `stock_import` names at most 20 ids (#1652): every id is a request on
//! the vendor's key, which every user of a hosted server shares, so a longer
//! list is refused before the library is asked anything.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use scorsese_core::{Fps, Project};
use scorsese_mcp::{Libraries, Stock, stocked_by, stocked_from};
use scorsese_providers::stock::{Candidate, Library, Medium, Page, Query, StockError};
use serde_json::json;

/// A library that has nothing, and counts how often it was asked.
#[derive(Default)]
struct Counting {
    asked: AtomicUsize,
}

impl Library for Counting {
    fn name(&self) -> &'static str {
        "Counting"
    }

    fn page(&self, _: &Query, _: u32) -> Result<Page, StockError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Ok(Page::default())
    }

    fn one(&self, _: Medium, _: u64) -> Result<Option<Candidate>, StockError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }

    fn hosts(&self) -> &'static [&'static str] {
        &[]
    }

    fn download(&self, url: &str, _: u64, _: &mut dyn std::io::Write) -> Result<u64, StockError> {
        unreachable!("nothing is there to download: {url}")
    }
}

#[test]
fn an_import_of_more_than_twenty_ids_asks_nothing() {
    let dir = std::env::temp_dir().join(format!("scorsese-stocking-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Project::new("stocking", Fps::THIRTY).save(&dir).unwrap();
    let counting = Arc::new(Counting::default());
    let library: Stock = counting.clone();
    let import = stocked_from(&library).remove(1);
    assert_eq!(import.name(), "stock_import");

    for kind in ["image", "lottie"] {
        let ids: Vec<u64> = (1..=21).collect();
        let arguments = json!({ "project": dir.to_string_lossy(), "id": ids, "kind": kind });
        let Err(refused) = import.call(&arguments) else {
            panic!("21 ids are refused");
        };
        assert!(refused.contains("at most 20"), "{refused}");
        assert_eq!(counting.asked.load(Ordering::SeqCst), 0, "{kind}");
    }

    let ids: Vec<u64> = (1..=20).collect();
    let arguments = json!({ "project": dir.to_string_lossy(), "id": ids, "kind": "image" });
    let said = import.call(&arguments).err().unwrap_or_default();
    assert_eq!(
        counting.asked.load(Ordering::SeqCst),
        20,
        "20 ids are each asked: {said}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The pair built with `stocked_by` asks it for each medium's library, and
/// answers from what it gives.
#[test]
fn stocked_by_asks_for_the_medium_of_the_call() {
    let dir = std::env::temp_dir().join(format!("scorsese-stocked-by-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Project::new("stocked-by", Fps::THIRTY).save(&dir).unwrap();
    let counting = Arc::new(Counting::default());
    let asked_for = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (given, seen) = (counting.clone(), asked_for.clone());
    let libraries: Libraries = Arc::new(move |medium| {
        seen.lock().unwrap().push(medium);
        Ok(given.clone() as Stock)
    });
    let tools = stocked_by(libraries, None);
    let names: Vec<&str> = tools.iter().map(|tool| tool.name()).collect();
    assert_eq!(names, ["stock_search", "stock_import"]);

    let project = dir.to_string_lossy();
    let search = json!({ "project": project, "query": "wave", "kind": "lottie" });
    tools[0].call(&search).expect("the library answers");
    assert_eq!(*asked_for.lock().unwrap(), [Medium::Lottie]);
    assert_eq!(counting.asked.load(Ordering::SeqCst), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A `.scor` received from someone else may carry `cache/` itself as a link
/// (#1718): a search keeps nothing where it leads.
#[cfg(unix)]
#[test]
fn a_search_keeps_nothing_through_a_linked_cache_folder() {
    let base = std::env::temp_dir().join(format!("scorsese-stock-linked-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (dir, outside) = (base.join("project"), base.join("outside"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    Project::new("stock-linked", Fps::THIRTY)
        .save(&dir)
        .unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("cache")).unwrap();
    let library: Stock = Arc::new(Counting::default());
    let search = stocked_from(&library).remove(0);

    let asked = json!({ "project": dir.to_string_lossy(), "query": "wave", "kind": "lottie" });
    let Err(refused) = search.call(&asked) else {
        panic!("kept through a link");
    };

    assert!(refused.contains("link"), "{refused}");
    let landed: Vec<_> = std::fs::read_dir(&outside).unwrap().collect();
    assert!(landed.is_empty(), "written through the link: {landed:?}");
    std::fs::remove_dir_all(&base).unwrap();
}
