//! One frame of the timeline, composited and handed back rather than encoded.
//!
//! A preview is a render with the encoder taken out. It plans the same way,
//! decodes the same way, draws slug cards the same way, and asks the same
//! [`scorsese_compositor::Compositor`] for pixels — the only difference is that
//! the frame that comes out is kept instead of written to a file. That is
//! deliberate to the point of being the reason this module is here at all: a
//! preview drawn by any other code would be a preview that can disagree with
//! the render, and then looking at it proves nothing.
//!
//! What it does *not* do is probe the project first. [`crate::run::Renderer`]
//! fills in missing media metadata because a clip's sound decides whether it is
//! mixed, and a still has no mix. The one measurement picture needs — how big a
//! `native` source is — [`Sizes`] takes itself, from the assets table when it is
//! there and ffprobe when it is not. The one measurement a held animation needs
//! — how long it is, so a still two seconds into a gif shows where the gif is
//! at two seconds — [`Loops`] takes from the file, because the assets table
//! never records it.

use std::path::Path;

use scorsese_compositor::Frame;
use scorsese_core::{Frames, Project};

use crate::error::RenderError;
use crate::held::Loops;
use crate::page::Chrome;
use crate::plan::{FrameRange, Plan};
use crate::preview::Preview;
use crate::raster::Sizes;
use crate::settings::RenderSettings;
use crate::tools::Tools;
use crate::workers::Workers;

use super::pages::Pages;

use super::segment::{Pass, Stage};

/// Composites the timeline at `at` into a frame of `settings.resolution`.
///
/// A frame past the last thing on the timeline is a [`crate::PlanError`] rather
/// than black: there is no such instant in the edit, and answering with a
/// picture would be inventing one.
pub(super) fn compose(
    tools: &Tools,
    chrome: Option<&Chrome>,
    settings: RenderSettings,
    preview: Option<&Preview>,
    project: &Project,
    project_root: &Path,
    at: Frames,
) -> Result<Frame, RenderError> {
    let plan = Plan::build(project, settings.fps, FrameRange::just(at))?;
    let sizes = Sizes::measure(tools, &plan, project_root)?;
    // A still is the case a held animation is most often opened part-way
    // through: scrubbing two seconds into a gif must show where the gif is at
    // two seconds, which the delivered file does.
    let loops = Loops::measure(tools, &plan, project_root);
    // A page on screen is captured whole, not just this instant: its clock
    // has to run from zero to get here, and the capture is cached for every
    // scrub after this one. What it noticed has nowhere to go from a still.
    let (pages, _) = Pages::capture(tools, chrome, &settings, &plan, project_root);
    let mut stage = Stage::new();
    let pass = Pass {
        tools,
        settings,
        plan: &plan,
        sizes: &sizes,
        loops: &loops,
        pages: &pages,
        project_root,
        // One frame has nothing to parallelise, and a still is one frame. The
        // pool would be capped to this anyway; saying it here means a scrub
        // never asks the machine how many threads it has.
        workers: Workers::new(1),
        preview,
    };

    // A one-frame range has one segment by construction: the cuts a plan splits
    // on are clip boundaries strictly inside the range, and a single frame has
    // no inside for one to fall in.
    let segment = plan
        .segments()
        .first()
        .expect("a plan over one frame has that frame's segment in it");

    let mut still = None;
    pass.render(segment, 1, &mut stage, &mut |frame| {
        // Cloned rather than borrowed out: the canvas is the `Stage`'s, and the
        // `Stage` dies with this function. One 8 MB copy per scrubbed frame is
        // nothing beside the decode that produced it.
        still = Some(frame.clone());
        Ok(())
    })?;
    Ok(still.expect("a pass over one frame composites exactly one frame"))
}
