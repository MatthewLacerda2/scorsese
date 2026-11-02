//! The mock Gemini: a still drawn now, or ordered in a batch that finishes
//! with it at the first asking — or stops, when the mock says so. Told to
//! [`say`](Mock::say) something, it answers in those words and draws nothing;
//! told to [`fail`](Mock::fail), it errors as the vendor does.

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
        self.unmade().map_or_else(|| Ok(still()), Err)
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
            None => Batch::Finished(vec![(key, self.unmade().map_or_else(|| Ok(still()), Err))]),
        })
    }
}

impl Mock {
    /// Why nothing is made, when the mock is told to make nothing.
    pub(super) fn unmade(&self) -> Option<ProviderError> {
        match (&self.say, &self.fail) {
            (Some(words), _) => Some(ProviderError::billed("mock image", words)),
            (None, Some(why)) => Some(ProviderError::new("mock image", why)),
            (None, None) => None,
        }
    }
}
