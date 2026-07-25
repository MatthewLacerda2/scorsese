//! Driving a plan through the pipes: decode → composite → encode.

use std::path::Path;

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Properties};
use scorsese_core::{AssetKind, Fps, Frames, Project};

use crate::error::RenderError;
use crate::pipe::{Decoder, Encoder, Source};
use crate::plan::{FrameRange, Plan, Segment, Shot};
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
        let mut stage = Stage::for_plan(&plan, self.settings);
        let mut written = 0;

        for segment in plan.segments() {
            let frames = plan.out_frames_of(segment);
            notes.extend(self.render_segment(
                segment,
                project_root,
                &plan,
                frames,
                &mut stage,
                &mut encoder,
            )?);
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

    /// Decodes one stretch of timeline, composites each of its frames, and
    /// hands them to the encoder.
    fn render_segment(
        &self,
        segment: &Segment<'_>,
        project_root: &Path,
        plan: &Plan<'_>,
        frames: u64,
        stage: &mut Stage,
        encoder: &mut Encoder,
    ) -> Result<Vec<Note>, RenderError> {
        // Split the borrow up front: the sources are written into and the canvas
        // read out of within the same frame, and they are separate buffers.
        let Stage {
            compositor,
            sources,
            canvas,
        } = stage;

        if segment.is_gap() {
            // No layers, so compositing nothing is exactly what a gap looks
            // like: the cleared canvas, once, written for as long as it lasts.
            compositor.composite(canvas, &[])?;
            for _ in 0..frames {
                encoder.write(canvas)?;
            }
            return Ok(Vec::new());
        }

        // One decoder per layer, all running at once. They are read in lockstep,
        // a frame from each per output frame, so every pipe drains evenly and
        // none of them blocks waiting for us.
        let mut decoders = Vec::with_capacity(segment.layers.len());
        for shot in &segment.layers {
            let source = source_for(shot, project_root, plan.timeline_fps(), frames)?;
            decoders.push(Decoder::start(self.tools, &source, &self.settings)?);
        }
        let mut missing = vec![0_u64; segment.layers.len()];

        for index in 0..frames {
            for (at, decoder) in decoders.iter_mut().enumerate() {
                if !decoder.read_into(&mut sources[at])? {
                    // This layer's source ran out before its clip did. Blank
                    // rather than black: an upper layer that went opaque black
                    // would paint over the tracks below it, which is not what
                    // running out of footage means.
                    sources[at].fill_transparent();
                    missing[at] += 1;
                }
            }

            // Keyframes are timed from each clip's own start, so that moving a
            // clip along the timeline never rewrites them. Which instant of the
            // timeline this output frame shows is the plan's to say.
            let at = plan.timeline_frame_of(segment, index);
            let layers: Vec<Layer<'_>> = segment
                .layers
                .iter()
                .zip(sources.iter())
                .map(|(shot, source)| Layer {
                    source,
                    properties: Properties::at(&shot.clip.keyframes, elapsed(at, shot.clip)),
                })
                .collect();

            compositor.composite(canvas, &layers)?;
            encoder.write(canvas)?;
        }

        for decoder in decoders {
            decoder.finish()?;
        }
        Ok(segment
            .layers
            .iter()
            .zip(missing)
            .filter(|(_, missing)| *missing > 0)
            .map(|(shot, missing)| Note::ClipRanShort {
                clip: shot.clip.id.to_string(),
                missing,
            })
            .collect())
    }
}

/// How far into its own clip a timeline frame is.
fn elapsed(at: Frames, clip: &scorsese_core::Clip) -> Frames {
    Frames(at.get().saturating_sub(clip.start.get()))
}

/// The buffers and the compositor a render reuses for every frame.
///
/// Allocated once and kept for the whole render: at 1080p30 a single frame
/// buffer is 8 MB, so allocating one per layer per frame would be hundreds of
/// megabytes a second of pure churn.
struct Stage {
    compositor: CpuCompositor,
    /// One decode buffer per layer, sized to the widest stack in the plan.
    sources: Vec<Frame>,
    /// What the encoder is about to be given.
    canvas: Frame,
}

impl Stage {
    fn for_plan(plan: &Plan<'_>, settings: RenderSettings) -> Self {
        Self {
            compositor: CpuCompositor::new(),
            sources: (0..plan.widest_stack())
                .map(|_| Frame::black(settings.resolution))
                .collect(),
            canvas: Frame::black(settings.resolution),
        }
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
