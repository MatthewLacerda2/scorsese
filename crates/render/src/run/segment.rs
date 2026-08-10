//! Rendering one stretch of timeline, over which the visible set does not
//! change.
//!
//! One decoder per layer that has a source, all running at once and read in
//! lockstep — a frame from each per output frame, so every pipe drains evenly
//! and none of them blocks waiting for us. **Not every layer has a source.** A
//! text asset carries its content in the document, and a prompt with nothing
//! generated has a slug card; both are drawn once, here, because their pixels
//! do not change over a clip and re-drawing every glyph thirty times a second
//! to get the same answer would be waste.

use std::path::Path;

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Properties};
use scorsese_core::{AssetKind, Clip, Fps, Frames};

use crate::error::RenderError;
use crate::pipe::{Decoder, Fitting, Source};
use crate::plan::{Plan, Segment, Shot};
use crate::raster::Sizes;
use crate::report::{Note, StandIn};
use crate::settings::RenderSettings;
use crate::slug::{self, Standing};
use crate::text::Painter;
use crate::tools::Tools;

/// Where a composited frame goes once it exists.
///
/// A render hands it to the encoder; a still keeps the one it asked for. The
/// destination is a callback rather than a type because the alternative is a
/// second walk over the plan that draws frames its own way — which is the one
/// thing a preview must never be. One compositing path, two things done with
/// what comes out of it.
pub(super) type Write<'a> = &'a mut dyn FnMut(&Frame) -> Result<(), RenderError>;

/// Everything a segment is rendered against: the tools, the settings, and the
/// three things worked out before any process was spawned.
pub(super) struct Pass<'a> {
    /// How ffmpeg is invoked, which is this crate's alone to know.
    pub(super) tools: &'a Tools,
    /// The raster, rate and quality being written.
    pub(super) settings: RenderSettings,
    /// The sequenced timeline, which says which instant each frame shows.
    pub(super) plan: &'a Plan<'a>,
    /// The measured size of every source wanted at its own size.
    pub(super) sizes: &'a Sizes,
    /// What the document's relative paths are relative to.
    pub(super) project_root: &'a Path,
}

impl Pass<'_> {
    /// Decodes one stretch of timeline, composites each of its frames, and
    /// hands them to `write`. What comes back is what the stretch noticed.
    pub(super) fn render(
        &self,
        segment: &Segment<'_>,
        frames: u64,
        stage: &mut Stage,
        write: Write<'_>,
    ) -> Result<Vec<Note>, RenderError> {
        // Split the borrow up front: the sources are written into and the canvas
        // read out of within the same frame, and they are separate buffers.
        let Stage {
            compositor,
            painter,
            sources,
            canvas,
        } = stage;

        if segment.is_gap() {
            // No layers, so compositing nothing is exactly what a gap looks
            // like: the cleared canvas, once, written for as long as it lasts.
            compositor.composite(canvas, &[])?;
            for _ in 0..frames {
                write(canvas)?;
            }
            return Ok(Vec::new());
        }

        let mut notes = Vec::new();
        let mut decoders: Vec<Option<Decoder>> = Vec::with_capacity(segment.layers.len());
        for (at, shot) in segment.layers.iter().enumerate() {
            decoders.push(self.begin(shot, &mut sources[at], painter, frames, &mut notes)?);
        }
        let mut missing = vec![0_u64; segment.layers.len()];

        for index in 0..frames {
            for (at, decoder) in decoders.iter_mut().enumerate() {
                let Some(decoder) = decoder else {
                    // Drawn once and still there: a text layer or a slug card
                    // never runs out of source, so there is nothing to read and
                    // nothing to report short.
                    continue;
                };
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
            let at = self.plan.timeline_frame_of(segment, index);
            let layers: Vec<Layer<'_>> = segment
                .layers
                .iter()
                .zip(sources.iter())
                .map(|(shot, source)| Layer {
                    source,
                    properties: Properties::at(shot.clip, elapsed(at, shot.clip)),
                    anchor: shot.clip.anchor,
                })
                .collect();

            compositor.composite(canvas, &layers)?;
            write(canvas)?;
        }

        for decoder in decoders.into_iter().flatten() {
            decoder.finish()?;
        }
        notes.extend(
            segment
                .layers
                .iter()
                .zip(missing)
                .filter(|(_, missing)| *missing > 0)
                .map(|(shot, missing)| Note::ClipRanShort {
                    clip: shot.clip.id.to_string(),
                    missing,
                }),
        );
        Ok(notes)
    }

    /// Gets one layer ready: a decoder to read from, or pixels drawn once and
    /// then left alone, in which case there is no decoder to hand back.
    fn begin(
        &self,
        shot: &Shot<'_>,
        buffer: &mut Frame,
        painter: &mut Painter,
        frames: u64,
        notes: &mut Vec<Note>,
    ) -> Result<Option<Decoder>, RenderError> {
        // Both of the drawn kinds are the size of the raster: a title is set at
        // whatever the render is, and a card is a panel of it.
        let drawn_at_raster = |buffer: &mut Frame| {
            if buffer.resolution() != self.settings.resolution {
                *buffer = Frame::black(self.settings.resolution);
            }
        };

        if shot.asset.kind == AssetKind::Text {
            drawn_at_raster(buffer);
            painter.paint(buffer, shot.asset, shot.clip.anchor, self.project_root)?;
            return Ok(None);
        }

        if shot.asset.kind == AssetKind::Color {
            drawn_at_raster(buffer);
            // Validation requires the colour, so the default is unreachable
            // through a loaded project. White rather than transparent if a
            // caller ever builds one in memory: a layer that silently vanished
            // would be harder to notice than one that is plainly the wrong
            // colour.
            buffer.fill(shot.asset.color.unwrap_or_default());
            return Ok(None);
        }

        let file = match slug::standing(shot, self.project_root)? {
            Standing::Media(file) => file,
            Standing::Card(absent) => {
                if absent.is_a_problem() {
                    notes.push(Note::GeneratedMissing {
                        clip: shot.clip.id.to_string(),
                        asset: shot.asset.id.to_string(),
                        stood_in: StandIn::SlugCard,
                    });
                }
                drawn_at_raster(buffer);
                slug::paint(buffer, shot.asset, absent);
                return Ok(None);
            }
        };

        let decoder = Decoder::start(
            self.tools,
            &source_for(
                shot,
                file,
                self.plan.timeline_fps(),
                frames,
                self.sizes.fitting(shot, self.settings.resolution),
            ),
            &self.settings,
        )?;
        // A source kept at its own size is not the size of the canvas, so the
        // buffer reading it is whatever this decoder produces — asked of the
        // decoder rather than worked out twice. Buffers are still reused across
        // segments; only a change of size costs one.
        if buffer.resolution() != decoder.raster() {
            *buffer = Frame::black(decoder.raster());
        }
        Ok(Some(decoder))
    }
}

/// How far into its own clip a timeline frame is.
fn elapsed(at: Frames, clip: &Clip) -> Frames {
    Frames(at.get().saturating_sub(clip.start.get()))
}

/// How to read a shot's media, given where it is.
fn source_for(
    shot: &Shot<'_>,
    file: std::path::PathBuf,
    timeline_fps: Fps,
    frames: u64,
    fitting: Fitting,
) -> Source {
    Source {
        file,
        // A still has no timeline of its own: it is held for the clip's
        // length rather than played, so there is nothing to seek into.
        still: shot.asset.kind == AssetKind::Image,
        seek_seconds: timeline_fps.seconds_at(shot.source_in),
        speed: shot.clip.speed,
        frames,
        fitting,
        // Read off the document, which is where a probe writes it. An asset
        // nobody probed reads as opaque — the same answer, and the same filter
        // chain, as before there was a field to read.
        has_alpha: shot
            .asset
            .media
            .and_then(|media| media.has_alpha)
            .unwrap_or(false),
        crop: shot.clip.crop,
    }
}

/// The buffers and the compositor a render reuses for every frame.
///
/// Allocated once and kept for the whole render: at 1080p30 a single frame
/// buffer is 8 MB, so allocating one per layer per frame would be hundreds of
/// megabytes a second of pure churn.
pub(super) struct Stage {
    /// Draws the frames.
    pub(super) compositor: CpuCompositor,
    /// Draws the text layers, and keeps any font it had to open off disk.
    pub(super) painter: Painter,
    /// One decode buffer per layer, sized to the widest stack in the plan.
    pub(super) sources: Vec<Frame>,
    /// What the encoder is about to be given.
    pub(super) canvas: Frame,
}

impl Stage {
    /// Buffers enough for the widest stack this plan ever composites.
    pub(super) fn for_plan(plan: &Plan<'_>, settings: RenderSettings) -> Self {
        Self {
            compositor: CpuCompositor::new(),
            painter: Painter::new(),
            sources: (0..plan.widest_stack())
                .map(|_| Frame::black(settings.resolution))
                .collect(),
            canvas: Frame::black(settings.resolution),
        }
    }
}
