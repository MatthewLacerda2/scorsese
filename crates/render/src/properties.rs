//! Everything this build can animate, and what to say about a keyframe track
//! that names something else.
//!
//! Two vocabularies meet here and nowhere else: the compositor publishes what
//! it draws, the mixer publishes what it hears, and each list sits beside the
//! code that implements it. This crate is simply the first place that can see
//! both, which is why the union is here rather than in either of them.
//!
//! **A warning, never a gate.** An unknown property does not fail a render, a
//! check, or a merge — the render it appears in produces exactly the file it
//! would have produced anyway. Per the gates-vs-signals rule this audits
//! quality rather than proving correctness, and a hard error would make every
//! newly animatable property a breaking change for a project authored against
//! a build that has it.

use scorsese_compositor::{ANIMATED as DRAWN, Registry};
use scorsese_core::Project;

use crate::audio::gain::ANIMATED as MIXED;
use crate::report::Note;

/// Every property something in this build animates.
pub const ANIMATABLE: Registry = Registry::new(&[DRAWN, MIXED]);

/// A keyframe track that nothing will ever read.
///
/// Which is what makes it worth saying: the track is perfectly valid, the
/// render succeeds, and the fade simply never happens. For a human that is a
/// confusing afternoon; for an agent working unattended the render "succeeded",
/// so nothing prompts a second look and the mistake ships.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown {
    pub clip: String,
    pub property: String,
    /// The known property it was probably meant to be, when there is an obvious
    /// candidate. A typo is the case being caught, so this is most of the value.
    pub did_you_mean: Option<&'static str>,
}

impl From<Unknown> for Note {
    fn from(unknown: Unknown) -> Self {
        Self::UnknownProperty {
            clip: unknown.clip,
            property: unknown.property,
            did_you_mean: unknown.did_you_mean,
        }
    }
}

/// Every keyframe track in the project naming a property nobody animates.
///
/// The whole project, not the range being rendered: a typo in a clip outside
/// today's `--range` is the same mistake, and only mentioning it when that clip
/// happens to be on screen would make the warning a matter of luck.
pub fn unknown_in(project: &Project) -> Vec<Unknown> {
    project
        .clips()
        .flat_map(|(_, clip)| {
            clip.keyframes
                .iter()
                .filter(|track| !ANIMATABLE.knows(&track.property))
                .map(|track| Unknown {
                    clip: clip.id.to_string(),
                    property: track.property.to_string(),
                    did_you_mean: ANIMATABLE
                        .did_you_mean(&track.property)
                        .map(|property| property.path),
                })
        })
        .collect()
}
