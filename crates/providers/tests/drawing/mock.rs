//! An image provider that costs nothing, and remembers every brief it was
//! handed — a cache that works is a cache that produces no second request.

use std::cell::RefCell;

use scorsese_providers::image::{Batch, Brief, ImageProvider, ProviderError};

/// A provider that draws `picture`, or refuses with `refusal` when there is one.
#[derive(Debug, Default)]
pub(crate) struct Mock {
    refusal: Option<String>,
    /// Every brief handed over, in order.
    pub(crate) drawn: RefCell<Vec<Brief>>,
    /// Every batch ordered: the keys each was sent with, its name its place.
    pub(crate) ordered: RefCell<Vec<Vec<String>>>,
    /// What asking after a batch answers: running while `None`, every picture
    /// on `Ok`, stopped in these words on `Err`.
    pub(crate) finished: RefCell<Option<Result<(), String>>>,
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

    fn order(&self, briefs: &[&Brief]) -> Result<String, ProviderError> {
        if let Some(message) = &self.refusal {
            return Err(ProviderError::new("Mock", message));
        }
        let mut ordered = self.ordered.borrow_mut();
        ordered.push(briefs.iter().map(|brief| brief.key()).collect());
        Ok(format!("batches/{}", ordered.len()))
    }

    fn ask(&self, operation: &str) -> Result<Batch, ProviderError> {
        let job: usize = operation
            .trim_start_matches("batches/")
            .parse()
            .expect("one of ours");
        Ok(match self.finished.borrow().clone() {
            None => Batch::Running,
            Some(Err(why)) => Batch::Stopped(why),
            Some(Ok(())) => Batch::Finished(
                self.ordered.borrow()[job - 1]
                    .iter()
                    .map(|key| (key.clone(), Ok(b"JPG".to_vec())))
                    .collect(),
            ),
        })
    }
}
