//! An image provider that costs nothing, and remembers every brief it was
//! handed — a cache that works is a cache that produces no second request.

use std::cell::RefCell;

use scorsese_providers::image::{Brief, ImageProvider, ProviderError};

/// A provider that draws `picture`, or refuses with `refusal` when there is one.
#[derive(Debug, Default)]
pub(crate) struct Mock {
    refusal: Option<String>,
    /// Every brief handed over, in order.
    pub(crate) drawn: RefCell<Vec<Brief>>,
}

impl Mock {
    /// A provider that draws anything.
    pub(crate) fn willing() -> Self {
        Self::default()
    }

    /// A provider that refuses everything, saying this.
    pub(crate) fn refusing(message: &str) -> Self {
        Self {
            refusal: Some(message.to_owned()),
            ..Self::default()
        }
    }

    /// How many briefs have been handed over.
    pub(crate) fn requests(&self) -> usize {
        self.drawn.borrow().len()
    }
}

impl ImageProvider for Mock {
    fn draw(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.drawn.borrow_mut().push(brief.clone());
        match &self.refusal {
            Some(message) => Err(ProviderError::new("Mock", message)),
            None => Ok(b"PNG".to_vec()),
        }
    }

    fn name(&self) -> &'static str {
        "Mock"
    }
}
