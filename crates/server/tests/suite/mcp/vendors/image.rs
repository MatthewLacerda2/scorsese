//! The mock Gemini: a still drawn now, or ordered in a batch that finishes
//! with it at the first asking — or stops, when the mock says so.

use std::sync::atomic::Ordering;

use scorsese_providers::image::{Batch, Brief, ImageProvider};
use scorsese_providers::video::ProviderError;

use super::{Mock, made};

/// The picture every still comes back as.
fn still() -> Vec<u8> {
    made(
        "still.jpg",
        &["-f", "lavfi", "-i", "testsrc=s=64x36", "-frames:v", "1"],
    )
}

impl ImageProvider for Mock {
    fn draw(&self, _: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.drawn.fetch_add(1, Ordering::SeqCst);
        Ok(still())
    }

    fn name(&self) -> &'static str {
        "mock image"
    }

    fn order(&self, briefs: &[&Brief]) -> Result<String, ProviderError> {
        self.ordered.fetch_add(1, Ordering::SeqCst);
        Ok(format!("batches/mock-{}", briefs[0].key()))
    }

    fn ask(&self, operation: &str) -> Result<Batch, ProviderError> {
        let key = operation.trim_start_matches("batches/mock-").to_owned();
        Ok(match &self.stop {
            Some(why) => Batch::Stopped(why.clone()),
            None => Batch::Finished(vec![(key, Ok(still()))]),
        })
    }
}
