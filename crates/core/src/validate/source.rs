//! Whether a clip asks for more of its source than there is — refused, or
//! only warned about.
//!
//! **Two answers to one question, and the line between them is history.** A
//! clip's [`Clip::source_end`] counts its speed, and until #1098 it did not:
//! a sped-up clip reading past its media passed validation, rendering
//! whatever the decoder gives at the end. A project that loads today must go
//! on loading, so that case is a warning ([`Project::past_source`]) and only
//! what was refused before, and still overruns, is refused now.

use std::fmt;

use crate::asset::{Asset, AssetId};
use crate::project::Project;
use crate::time::{Fps, Frames};
use crate::timeline::{Clip, ClipId};

use super::error::TimelineProblem;

/// A clip whose speed carries it past the end of its source, which validation
/// accepts: the render plays what there is and then whatever the decoder hands
/// back after it, and that is almost certainly not what was meant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastSource {
    /// The sped-up clip.
    pub clip: ClipId,
    /// The asset that runs out under it.
    pub asset: AssetId,
    /// The source frame it plays up to, `source_in` and speed included.
    pub reaches: Frames,
    /// How much source there is, on the timeline's grid.
    pub available: Frames,
}

impl fmt::Display for PastSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "clip `{}` is sped up to reach {} into asset `{}`, which is {} long, so its end \
             plays past the footage; shorten it or slow it down",
            self.clip, self.reaches, self.asset, self.available
        )
    }
}

impl Project {
    /// Every clip, groups' members included, that [`PastSource`] describes —
    /// the half of an overrun loading accepts. The other half is
    /// [`TimelineProblem::ClipOutlastsSource`].
    pub fn past_source(&self) -> Vec<PastSource> {
        self.every_clip()
            .filter_map(|(_, clip)| {
                let asset = self.asset(&clip.asset)?;
                match overrun(self.timeline_fps, asset, clip)? {
                    Overrun::Warned(found) => Some(found),
                    Overrun::Refused(_) => None,
                }
            })
            .collect()
    }
}

/// A clip cannot show footage nobody shot: where it starts in the source plus
/// what it plays through has to land inside the source's own length.
///
/// The one check here that reads something measured rather than something
/// written, and so the one that can only be made when a measurement exists.
/// [`Asset::length`] answers `None` for everything with no inherent length —
/// a still, a title, a colour, a sketch with no file yet — and for a file
/// nobody has probed, and an absent ceiling bounds nothing. That is the rule
/// being honest about what it knows: the alternative, guessing a length, would
/// refuse honest edits on assets we have simply not looked at.
///
/// A **synthesised** asset is the other one it leaves alone, though it carries
/// a measured length: that length is its *last bake's*, and the document
/// cannot say whether the bake is still the one its recipe makes. Writing a
/// recipe longer, or lengthening the clip a song is fitted to, makes the bake
/// stale by arithmetic (`docs/recipes.md`) — and holding the clip to the old
/// length would refuse exactly the edit the new recipe is for, in every tool
/// that opens the project, the bake among them (#1062). Only the synthesis
/// side can read the recipe and tell, so the render says it there
/// (`scorsese_providers::synth::out_of_date`), as a note.
pub(super) fn check_source_range(
    fps: Fps,
    asset: &Asset,
    clip: &Clip,
    errors: &mut Vec<TimelineProblem>,
) {
    if let Some(Overrun::Refused(problem)) = overrun(fps, asset, clip) {
        errors.push(problem);
    }
}

enum Overrun {
    Refused(TimelineProblem),
    Warned(PastSource),
}

/// How a clip that reads past its source is answered, if it does.
///
/// Refused when it overruns whichever way it is counted — by its speed, and by
/// the one source frame per timeline frame validation counted before #1098 —
/// so a slowed-down clip is held to what it really reads and a sped-up one is
/// refused only where it always was.
fn overrun(fps: Fps, asset: &Asset, clip: &Clip) -> Option<Overrun> {
    if asset.kind.is_synthesized() {
        return None;
    }
    let available = asset.length(fps)?;
    let reaches = clip.source_end();
    if reaches <= available {
        return None;
    }
    if (clip.source_in + clip.duration).min(reaches) > available {
        return Some(Overrun::Refused(TimelineProblem::ClipOutlastsSource {
            clip: clip.id.clone(),
            asset: asset.id.clone(),
            reaches,
            available,
        }));
    }
    Some(Overrun::Warned(PastSource {
        clip: clip.id.clone(),
        asset: asset.id.clone(),
        reaches,
        available,
    }))
}
