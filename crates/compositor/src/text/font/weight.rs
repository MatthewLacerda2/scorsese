//! Which instance of a variable face to draw.
//!
//! Split from the face itself because it answers a different question: not
//! *how do I get an outline* but *which of the outlines this file can produce
//! did the document ask for*. It is also the half that refuses — a face and a
//! weight can disagree, and every way they can is decided here.

use skrifa::instance::Location;
use skrifa::{FontRef, MetadataProvider, Tag};

use super::FontError;

/// The variation axis that means "how heavy", as OpenType spells it.
const WEIGHT_AXIS: Tag = Tag::new(b"wght");

/// Turns a weight into a position in the file's variation space, refusing
/// every pairing of file and weight that cannot mean one thing.
///
/// A file with no `wght` axis is static as far as this is concerned — a face
/// varying only on `opsz` or `wdth` has one weight like any static one, and
/// naming a weight for it is the same mistake.
pub(super) fn locate(font: &FontRef<'_>, weight: Option<u16>) -> Result<Location, FontError> {
    let axes = font.axes();
    let axis = axes.get_by_tag(WEIGHT_AXIS);
    match (axis, weight) {
        (None, None) => Ok(Location::default()),
        (None, Some(weight)) => Err(FontError::StaticWithWeight { weight }),
        (Some(axis), None) => Err(FontError::VariableWithoutWeight {
            min: axis.min_value(),
            default: axis.default_value(),
            max: axis.max_value(),
        }),
        (Some(axis), Some(weight)) => {
            let asked = f32::from(weight);
            if asked < axis.min_value() || asked > axis.max_value() {
                return Err(FontError::WeightOffAxis {
                    weight,
                    min: axis.min_value(),
                    max: axis.max_value(),
                });
            }
            Ok(axes.location([(WEIGHT_AXIS, asked)]))
        }
    }
}
