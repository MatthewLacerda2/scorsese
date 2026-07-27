//! Driving a plan through the pipes: decode → composite → encode.

use std::path::Path;

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Properties};
use scorsese_core::{AssetKind, Fps, Frames, Project};

use crate::audio;
use crate::error::RenderError;
use crate::pipe::{Decoder, Encoder, Fitting, Source};
use crate::plan::{FrameRange, Plan, Segment, Shot};
use crate::raster::Sizes;
use crate::report::{Note, RenderReport};
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// Renders projects with one set of settings.
pub struct Renderer<'a> {
    tools: &'a Tools,
    settings: RenderSettings,
}

impl<'a> Renderer<'a> {
    /// Borrows the tools rather than discovering its own, so the cost of
    /// checking ffmpeg is paid once however many renders follow.
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
        // What a video clip's file has on it decides whether its sound is
        // mixed, so anything the project never recorded is found out here —
        // before the plan, which is a pure function of the document.
        let (project, probe_notes) = crate::probe::fill_media(self.tools, project, project_root);
        let plan = Plan::build(&project, self.settings.fps, range)?;
        let mut notes = plan.notes().to_vec();
        notes.extend(probe_notes);
        // Said at the start, about the whole project rather than the range: a
        // keyframe track nothing resolves does not stop this render, it just
        // never does anything, and that is the sort of thing a render should
        // mention before it spends minutes producing exactly the wrong fade.
        notes.extend(
            crate::properties::unknown_in(&project)
                .into_iter()
                .map(Note::from),
        );
        // Before anything is spawned: a clip asking for its source's own size
        // needs that size established, and this is the cheap place to fail if
        // it cannot be. What the probe above filled in is answer enough for
        // most of them, so this rarely spawns anything of its own.
        let sizes = Sizes::measure(self.tools, &plan, project_root)?;

        // Sound before picture, because the encoder needs the finished mix as
        // an input file. It is also the cheaper half: a mix that fails on a
        // missing music file should fail before we spend minutes encoding.
        let mixed = audio::mixdown(self.tools, &self.settings, &plan, project_root, out)?;
        let mix = mixed.as_ref().map(|(mixdown, _)| mixdown.path());
        let has_audio = mix.is_some();
        if let Some((_, mix_notes)) = &mixed {
            notes.extend(mix_notes.iter().cloned());
        }

        let mut encoder = Encoder::start(self.tools, &self.settings, mix, out)?;
        let mut stage = Stage::for_plan(&plan, self.settings);
        let mut written = 0;

        for segment in plan.segments() {
            let frames = plan.out_frames_of(segment);
            notes.extend(self.render_segment(
                segment,
                project_root,
                &plan,
                &sizes,
                frames,
                &mut stage,
                &mut encoder,
            )?);
            written += frames;
        }

        encoder.finish()?;
        // Only now is the scratch mix expendable: dropping it removes the file,
        // and the encoder has been reading from it until this point.
        drop(mixed);
        Ok(RenderReport {
            frames: written,
            fps: self.settings.fps,
            resolution: self.settings.resolution,
            seconds_of_audio: has_audio.then(|| {
                plan.total_samples(self.settings.sample_rate.hz()) as f64
                    / f64::from(self.settings.sample_rate.hz())
            }),
            notes,
        })
    }

    /// Decodes one stretch of timeline, composites each of its frames, and
    /// hands them to the encoder.
    #[allow(clippy::too_many_arguments)]
    fn render_segment(
        &self,
        segment: &Segment<'_>,
        project_root: &Path,
        plan: &Plan<'_>,
        sizes: &Sizes,
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
        for (at, shot) in segment.layers.iter().enumerate() {
            let source = source_for(
                shot,
                project_root,
                plan.timeline_fps(),
                frames,
                sizes.fitting(shot),
            )?;
            let decoder = Decoder::start(self.tools, &source, &self.settings)?;
            // A source kept at its own size is not the size of the canvas, so
            // the buffer reading it is whatever this decoder produces — asked
            // of the decoder rather than worked out twice. Buffers are still
            // reused across segments; only a change of size costs one.
            if sources[at].resolution() != decoder.raster() {
                sources[at] = Frame::black(decoder.raster());
            }
            decoders.push(decoder);
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
    fitting: Fitting,
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
        fitting,
    })
}
