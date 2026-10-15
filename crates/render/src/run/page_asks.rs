//! What a renderer says about the pages it shows: the captures rendering a
//! project would need ([`Renderer::page_requests`], [`Renderer::page_captures`])
//! and whether those captures are being made elsewhere right now
//! ([`Renderer::capture_underway`], #1202).

use std::path::Path;

use scorsese_core::Project;

use super::pages::{requests, wanted};
use crate::Renderer;
use crate::error::RenderError;
use crate::page;
use crate::plan::{FrameRange, Plan};

impl Renderer<'_> {
    /// Every page capture rendering `project` at these settings would need,
    /// once each — what to hand [`crate::page::capture`] ahead of time. The
    /// project's folder is where a narration's word timings are read from
    /// (#811), so it has to be the one the render will be given.
    pub fn page_requests(
        &self,
        project: &Project,
        project_root: &Path,
    ) -> Result<Vec<page::Request>, RenderError> {
        let plan = Plan::build(project, self.settings.fps, FrameRange::ALL)?;
        Ok(requests(&self.settings, &plan, project_root))
    }

    /// [`Renderer::page_requests`], with the stretch of each page a render
    /// shows: what a caller capturing ahead of a render that does not capture
    /// ([`Renderer::without_capturing`]) hands [`crate::page::capture_frames`],
    /// so only the frames on screen are drawn (#809, #890). A clip's stretch is
    /// the frames it shows over the whole timeline, a frame or two wider for
    /// the decoder, whatever `range` the render is later given.
    pub fn page_captures(
        &self,
        project: &Project,
        project_root: &Path,
    ) -> Result<Vec<page::Wanted>, RenderError> {
        let plan = Plan::build(project, self.settings.fps, FrameRange::ALL)?;
        Ok(wanted(&self.settings, &plan, project_root))
    }

    /// Says that the pages this renderer does not capture are being captured
    /// elsewhere right now, so a page without a capture is drawn as a card
    /// saying it is being captured rather than "not captured" (#1202). Only
    /// meaningful with [`Renderer::without_capturing`]; a failed or unstarted
    /// capture is left as the latter.
    pub fn capture_underway(self, underway: bool) -> Self {
        Self { underway, ..self }
    }
}
