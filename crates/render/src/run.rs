//! Driving a plan through the pipes: decode → composite → encode.

use std::path::Path;

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Properties};
use scorsese_core::{AssetKind, Fps, Frames, Project};

use crate::error::RenderError;
use crate::pipe::{Decoder, Encoder, Source};
use crate::plan::{Content, FrameRange, Plan, Segment, Shot};
use crate::report::{Note, RenderReport};
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// Renders projects with one set of settings.
pub struct Renderer<'a> {
    tools: &'a Tools,
    settings: RenderSettings,
}

impl<'a> Renderer<'a> {
    pub fn new(tools: &'a Tools, settings: RenderSettings) -> Self {
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
        let mut stage = Stage {
            compositor: CpuCompositor::new(),
            source: Frame::black(self.settings.resolution),
            canvas: Frame::black(self.settings.resolution),
        };
        let mut written = 0;

        for segment in plan.segments() {
            let frames = plan.out_frames_of(segment);
            match &segment.content {
                Content::Gap => {
                    // A gap has no layers, so compositing nothing is exactly
                    // what it looks like: the cleared canvas.
                    stage.compositor.composite(&mut stage.canvas, &[])?;
                    for _ in 0..frames {
                        encoder.write(&stage.canvas)?;
                    }
                }
                Content::Shot(shot) => {
                    let note = self.render_shot(
                        shot,
                        project_root,
                        &plan,
                        segment,
                        frames,
                        &mut stage,
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

    /// Decodes one clip's frames, composites each, and hands them to the encoder.
    #[allow(clippy::too_many_arguments)]
    fn render_shot(
        &self,
        shot: &Shot<'_>,
        project_root: &Path,
        plan: &Plan<'_>,
        segment: &Segment<'_>,
        frames: u64,
        stage: &mut Stage,
        encoder: &mut Encoder,
    ) -> Result<Option<Note>, RenderError> {
        let source = source_for(shot, project_root, plan.timeline_fps(), frames)?;
        let mut decoder = Decoder::start(self.tools, &source, &self.settings)?;
        let mut missing = 0;

        for index in 0..frames {
            if !decoder.read_into(&mut stage.source)? {
                // The source ran out before the clip did. Black is the honest
                // answer, and the note below makes sure it is not a silent one.
                stage.source.fill_black();
                missing += 1;
            }

            // Keyframes are timed from the clip's own start, so that moving a
            // clip along the timeline never rewrites them. Which instant of the
            // clip this output frame shows is the plan's to say.
            let at = plan.timeline_frame_of(segment, index);
            let elapsed = Frames(at.get().saturating_sub(shot.clip.start.get()));
            let properties = Properties::at(&shot.clip.keyframes, elapsed);

            stage.compositor.composite(
                &mut stage.canvas,
                &[Layer {
                    source: &stage.source,
                    properties,
                }],
            )?;
            encoder.write(&stage.canvas)?;
        }

        decoder.finish()?;
        Ok((missing > 0).then(|| Note::ClipRanShort {
            clip: shot.clip.id.to_string(),
            missing,
        }))
    }
}

/// The buffers and the compositor a render reuses for every frame.
///
/// Held together because they are allocated once and live for the whole render:
/// at 1080p30, allocating these per frame would be a few hundred megabytes a
/// second of pure churn.
struct Stage {
    compositor: CpuCompositor,
    /// What the decoder just handed us.
    source: Frame,
    /// What the encoder is about to be given.
    canvas: Frame,
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
