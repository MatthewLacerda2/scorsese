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

mod draw;
mod layers;
mod pipeline;
mod redraw;
mod source;

use std::path::Path;

use scorsese_compositor::{CpuCompositor, Frame};

use crate::error::RenderError;
use crate::follow;
use crate::held::Loops;
use crate::plan::{Plan, Segment};
use crate::preview::Preview;
use crate::raster::Sizes;
use crate::report::Note;
use crate::settings::RenderSettings;
use crate::text::Painter;
use crate::tools::Tools;
use crate::workers::Workers;

use layers::{Among, Pixels};
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
    /// The length of every held animation a stretch opens part-way through.
    pub(super) loops: &'a Loops,
    /// What the document's relative paths are relative to.
    pub(super) project_root: &'a Path,
    /// How many frames may be composited at once.
    pub(super) workers: Workers,
    /// What makes this a preview rather than a delivery, when it is one.
    pub(super) preview: Option<&'a Preview>,
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
        // Groups opened: a group clip's members are layers like any other,
        // drawn into the group's canvas rather than onto the frame.
        let entries = layers::open(&segment.layers);
        let mut slots = Vec::with_capacity(entries.len());
        let mut decoders = Vec::new();
        // Counted separately from the decoders: a drawn layer's buffer is the
        // size of the raster and is filled by drawing rather than reading, so
        // the two sets of buffers are allocated on different grounds even
        // though a job holds them in one list. A group's canvas is a third
        // count, kept in a list of its own — see [`pipeline`] for why.
        let (mut drawn, mut composed) = (0, 0);
        for entry in &entries {
            let (slot, decoder) = self.begin(
                entry.shot,
                painter,
                frames,
                Among {
                    live: decoders.len(),
                    drawn,
                    composed,
                    within: entry.within,
                    layers: &entries,
                },
                &mut notes,
            )?;
            match slot.pixels {
                Pixels::Drawn { .. } | Pixels::Typed { .. } => drawn += 1,
                Pixels::Composed { .. } => composed += 1,
                Pixels::Held(_) | Pixels::Live(_) => {}
            }
            slots.push(slot);
            decoders.extend(decoder);
        }

        // Mattes last, when every slot exists: a matte's slot comes after
        // the one it masks, so the link can only be made looking back.
        for (at, entry) in entries.iter().enumerate() {
            let Some(masked) = entry.serves else {
                continue;
            };
            let invert = entries[masked]
                .shot
                .matte
                .as_ref()
                .is_some_and(|matte| matte.invert);
            slots[masked].matte = Some((at, invert));
            slots[at].is_matte = true;
        }

        // Once every layer is ready, since a follower's arrow may come after
        // it in drawing order.
        // Each layer as [`follow`] sees it: the same resolution `describe --at`
        // places a follower by, so the two cannot disagree.
        let placing: Vec<follow::Layer<'_>> = entries
            .iter()
            .zip(&slots)
            .map(|(entry, slot)| follow::Layer {
                clip: entry.shot.clip,
                shape: entry.shot.asset.shape.as_ref(),
                within: entry.within,
                rect: slot.rect,
            })
            .collect();
        let (riders, lost) = follow::riders(&placing, self.plan.project());
        notes.extend(
            lost.into_iter()
                .map(|(clip, arrow)| Note::FollowLost { clip, arrow }),
        );

        let missing = pipeline::drive(
            self,
            segment,
            frames,
            Parts {
                slots: &slots,
                entries: &entries,
                placing: &placing,
                riders: &riders,
                drawn,
                composed,
                decoders: &mut decoders,
                compositors: &mut compositors[..workers],
                pools,
            },
            write,
        )?;

        for decoder in decoders {
            decoder.finish()?;
        }
        notes.extend(
            entries
                .iter()
                .zip(&slots)
                .filter_map(|(entry, slot)| match slot.pixels {
                    // A layer drawn — once, or afresh every frame — never runs out of
                    // source, so there is nothing to read and nothing to report
                    // short. A group is drawn too; a member of it that runs short is
                    // reported as itself.
                    Pixels::Held(_)
                    | Pixels::Drawn { .. }
                    | Pixels::Typed { .. }
                    | Pixels::Composed { .. } => None,
                    Pixels::Live(at) => (missing[at] > 0).then(|| Note::ClipRanShort {
                        clip: entry.shot.clip.id.to_string(),
                        missing: missing[at],
                    }),
                }),
        );
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
