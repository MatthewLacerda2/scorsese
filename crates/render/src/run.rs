//! Driving a plan through the pipes: decode → composite → encode.

use std::path::Path;

use scorsese_core::{AssetKind, Fps, Project};

use crate::error::RenderError;
use crate::frame::Frame;
use crate::pipe::{Decoder, Encoder, Source};
use crate::plan::{Content, FrameRange, Plan, Shot};
use crate::report::{Note, RenderReport};
use crate::settings::RenderSettings;

/// Renders projects with one set of settings.
pub struct Renderer<'a> {
    tools: &'a crate::tools::Tools,
    settings: RenderSettings,
}

impl<'a> Renderer<'a> {
    pub fn new(tools: &'a crate::tools::Tools, settings: RenderSettings) -> Self {
        Self { tools, settings }
    }

    /// Renders `range` of `project` to `out`.
    ///
    /// Expects a project that already validated — [`Project::load`] does that,
    /// and rendering an incoherent timeline is not a thing worth defining.
    pub fn render(
        &self,
        project: &Project,
        project_root: &Path,
        range: FrameRange,
        out: &Path,
    ) -> Result<RenderReport, RenderError> {
        let plan = Plan::build(project, self.settings.fps, range)?;
        let mut notes = plan.notes().to_vec();
        let mut encoder = Encoder::start(self.tools, &self.settings, out)?;
        let mut frame = Frame::black(self.settings.resolution);
        let mut written = 0;

        for segment in plan.segments() {
            let frames = plan.out_frames_of(segment);
            match &segment.content {
                Content::Gap => {
                    frame.fill_black();
                    for _ in 0..frames {
                        encoder.write(&frame)?;
                    }
                }
                Content::Shot(shot) => {
                    let note = self.render_shot(
                        shot,
                        project_root,
                        project.timeline_fps,
                        frames,
                        &mut frame,
                        &mut encoder,
                    )?;
                    notes.extend(note);
                }
            }
            written += frames;
        }

        encoder.finish()?;
        Ok(RenderReport {
            frames: written,
            fps: self.settings.fps,
            resolution: self.settings.resolution,
            notes,
        })
    }

    /// Decodes one clip's worth of frames and hands each to the encoder.
    fn render_shot(
        &self,
        shot: &Shot<'_>,
        project_root: &Path,
        timeline_fps: Fps,
        frames: u64,
        frame: &mut Frame,
        encoder: &mut Encoder,
    ) -> Result<Option<Note>, RenderError> {
        let source = source_for(shot, project_root, timeline_fps, frames)?;
        let mut decoder = Decoder::start(self.tools, &source, &self.settings)?;
        let mut missing = 0;

        for _ in 0..frames {
            if !decoder.read_into(frame)? {
                // The source ran out before the clip did. Black is the honest
                // answer, and the note below makes sure it is not a silent one.
                frame.fill_black();
                missing += 1;
            }

            // This is the compositing seam. Today one decoded frame becomes one
            // output frame untouched, which is why there is nothing between the
            // read and the write. Compositor v1 replaces this line with the
            // real thing: several source frames, transforms, opacity, and text
            // resolved into one output frame. Nothing on either side of it —
            // the plan, the decode pipe, the encode pipe — changes when it
            // does, which is the point of building the spine first.

            encoder.write(frame)?;
        }

        decoder.finish()?;
        Ok((missing > 0).then(|| Note::ClipRanShort {
            clip: shot.clip.id.to_string(),
            missing,
        }))
    }
}

/// Where a shot's media is on disk, and how to read it.
fn source_for(
    shot: &Shot<'_>,
    project_root: &Path,
    timeline_fps: Fps,
    frames: u64,
) -> Result<Source, RenderError> {
    let path = shot
        .asset
        .path
        .as_ref()
        .expect("the plan refuses a shot whose asset has no path");
    let file = path.resolve(project_root);
    if !file.is_file() {
        return Err(RenderError::MissingMedia {
            asset: shot.asset.id.to_string(),
            path: file,
        });
    }
    Ok(Source {
        file,
        // A still has no timeline of its own: it is held for the clip's
        // length rather than played, so there is nothing to seek into.
        still: shot.asset.kind == AssetKind::Image,
        seek_seconds: timeline_fps.seconds(shot.source_in),
        frames,
    })
}
