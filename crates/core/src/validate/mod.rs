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
//!
//! Almost all of it is about the *document* and stops at the edge of it: a
//! path is checked for shape, never for a file being there. The exception is
//! narrow and named — a clip is held to the length of the media its asset was
//! **measured** to be, when a measurement exists — and it is not the document
//! reaching for the disk: it reads the `media` block the pool already wrote.

mod assets;
mod error;
mod field;
mod report;
mod speech;
mod timeline;
mod video;

pub use error::{AssetProblem, SpeechProblem, TimelineProblem, ValidationError, VideoProblem};
pub use field::AssetField;
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
        // The script is a path like any other, so it obeys the rules any other
        // path obeys — checked here rather than under `assets`, because it
        // belongs to the document and there is no asset to name in the message.
        if let Some(script) = &self.script
            && let Err(problem) = script.check()
        {
            errors.push(ValidationError::BadScriptPath {
                path: script.clone(),
                problem,
            });
        }
        errors.extend(assets::check(self).into_iter().map(Into::into));
        errors.extend(timeline::check(self).into_iter().map(Into::into));

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
