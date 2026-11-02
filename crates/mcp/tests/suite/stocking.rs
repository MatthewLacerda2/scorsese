//! One `stock_import` names at most 20 ids (#1652): every id is a request on
//! the vendor's key, which every user of a hosted server shares, so a longer
//! list is refused before the library is asked anything.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use scorsese_core::{Fps, Project};
use scorsese_mcp::{Stock, stocked_from};
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
