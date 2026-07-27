//! The collected result of a validation pass.
//!
//! Separate from the list of things that can be wrong because it is a
//! different job: [`ValidationError`] says what one problem is, and this says
//! how a whole document's worth of them reads.

use std::fmt;

use super::error::ValidationError;

/// Everything wrong with a project, collected in one pass.
///
/// Validation reports all problems together rather than stopping at the
/// first: an agent fixing a project unattended should see the whole list, not
/// discover it one round-trip at a time.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationErrors(Vec<ValidationError>);

impl ValidationErrors {
    pub(crate) fn new(errors: Vec<ValidationError>) -> Self {
        Self(errors)
    }

    /// The problems, in the order validation found them: assets first, then
    /// tracks and clips.
    pub fn as_slice(&self) -> &[ValidationError] {
        &self.0
    }

    /// How many problems there are — what the `Display` line above the list
    /// counts.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when the project is valid. Never observed on a value that reached
    /// a caller: validation returns `Ok` rather than an empty error set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Takes the problems out, for a caller that wants to sort or filter them
    /// rather than print them.
    pub fn into_vec(self) -> Vec<ValidationError> {
        self.0
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = if self.0.len() == 1 {
            "problem"
        } else {
            "problems"
        };
        write!(f, "{} {plural} in this project:", self.0.len())?;
        for error in &self.0 {
            write!(f, "\n  - {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
