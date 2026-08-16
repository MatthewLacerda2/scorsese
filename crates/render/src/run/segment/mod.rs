//! Rendering one stretch of timeline, over which the visible set does not
//! change.
//!
//! One decoder per layer that has a source, all running at once and read in
//! lockstep — a frame from each per output frame, so every pipe drains evenly
//! and none of them blocks waiting for us. Which layers those are, and what the
//! rest of them are drawn from instead, is [`layers`]; what happens to a frame
//! between the decoders and the encoder is [`pipeline`], which is where the
//! only parallel stage of a render lives.
//!
//! This module is what the two have in common: the settings and the plan they
//! are worked out against, and the buffers they are worked out in.

mod attach;
mod layers;
mod pipeline;

use std::path::Path;

use scorsese_compositor::{CpuCompositor, Frame};

use crate::error::RenderError;
use crate::plan::{Plan, Segment};
use crate::raster::Sizes;
use crate::report::Note;
use crate::settings::RenderSettings;
use crate::text::Painter;
use crate::tools::Tools;
use crate::workers::Workers;

use layers::Pixels;
use pipeline::{Parts, Pools};

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
    /// How many frames may be composited at once.
    pub(super) workers: Workers,
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
        // Split the borrow up front: a layer is painted with one of these while
        // frames are drawn with the others, and they are separate things.
        let Stage {
            compositors,
            painter,
            pools,
        } = stage;
        // Never more workers than there are frames for them: a still is one
        // frame, and a thread spawned to draw nothing costs more than it saved.
        let workers = self
            .workers
            .get()
            .min(usize::try_from(frames).unwrap_or(usize::MAX))
            .max(1);
        if compositors.len() < workers {
            compositors.resize_with(workers, CpuCompositor::new);
        }

        if segment.is_gap() {
            pipeline::gap(
                &mut compositors[0],
                self.settings.resolution,
                frames,
                pools,
                write,
            )?;
            return Ok(Vec::new());
        }

        let mut notes = Vec::new();
        let mut slots = Vec::with_capacity(segment.layers.len());
        let mut decoders = Vec::new();
        // Counted separately from the decoders: a drawn layer's buffer is the
        // size of the raster and is filled by drawing rather than reading, so
        // the two sets of buffers are allocated on different grounds even
        // though a job holds them in one list.
        let mut drawn = 0;
        for shot in &segment.layers {
            let (slot, decoder) = self.begin(
                shot,
                painter,
                frames,
                decoders.len(),
                drawn,
                &segment.layers,
                &mut notes,
            )?;
            if matches!(slot.pixels, Pixels::Drawn { .. }) {
                drawn += 1;
            }
            slots.push(slot);
            decoders.extend(decoder);
        }

        let missing = pipeline::drive(
            self,
            segment,
            frames,
            Parts {
                slots: &slots,
                drawn,
                decoders: &mut decoders,
                compositors: &mut compositors[..workers],
                pools,
            },
            write,
        )?;

        for decoder in decoders {
            decoder.finish()?;
        }
        notes.extend(segment.layers.iter().zip(&slots).filter_map(
            |(shot, slot)| match slot.pixels {
                // A layer drawn — once, or afresh every frame — never runs out of
                // source, so there is nothing to read and nothing to report
                // short.
                Pixels::Held(_) | Pixels::Drawn { .. } => None,
                Pixels::Live(at) => (missing[at] > 0).then(|| Note::ClipRanShort {
                    clip: shot.clip.id.to_string(),
                    missing: missing[at],
                }),
            },
        ));
        Ok(notes)
    }
}

/// The compositors and the buffers a render reuses for every frame.
///
/// Allocated once and kept for the whole render: at 1080p30 a single frame
/// buffer is 8 MB, so allocating one per layer per frame would be hundreds of
/// megabytes a second of pure churn.
#[derive(Default)]
pub(super) struct Stage {
    /// One per worker, grown as the first segment that needs them arrives.
    /// Kept rather than made per segment because each carries the scratch a
    /// graded or translucent layer is drawn through.
    compositors: Vec<CpuCompositor>,
    /// Draws the text layers, and keeps any font it had to open off disk.
    painter: Painter,
    /// Every frame buffer in flight, and where a spent one goes back to.
    pools: Pools,
}

impl Stage {
    /// Nothing allocated yet: what a segment needs depends on what is in it,
    /// and the first one that wants a buffer is what sizes it.
    pub(super) fn new() -> Self {
        Self::default()
    }
}
