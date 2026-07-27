//! Project validation.
//!
//! Validation answers one question: is this project coherent enough to render
//! and to generate from? It checks what the type system cannot — that clip
//! references resolve, that paths stay inside the project, that clips do not
//! fight over the same instant of a track, that an asset carries the fields
//! its kind requires.
//!
//! Every check runs and every problem is collected. Reporting all of them at
//! once is the point: an agent repairing a project unattended should get the
//! whole list, not one error per round-trip.

mod assets;
mod error;
mod report;
mod timeline;

pub use error::ValidationError;
pub use report::ValidationErrors;

use crate::project::{Project, SCHEMA_VERSION};

impl Project {
    /// Checks the project and returns every problem found.
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if self.schema_version != SCHEMA_VERSION {
            errors.push(ValidationError::UnsupportedSchemaVersion {
                found: self.schema_version,
                supported: SCHEMA_VERSION,
            });
        }
        assets::check(self, &mut errors);
        timeline::check(self, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::new(errors))
        }
    }

    /// True when the project has no validation problems.
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}
