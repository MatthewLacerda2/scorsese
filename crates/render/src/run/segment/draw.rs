//! The parallel stage: a worker takes a frame's job, composites it, and hands
//! it back.
//!
//! Split from [`super::pipeline`], which feeds and drains these workers, because
//! drawing one frame is a concern of its own once there are groups in it: the
//! frame is not one composite but several — each group's members into the
//! group's canvas, innermost first, and then everything onto the frame — and
//! where each layer's pixels live for that frame is a small map of its own
//! ([`Sources`]).

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender};

use scorsese_compositor::{CompositeError, Compositor, CpuCompositor, Frame, Layer, Properties};

use crate::error::RenderError;

use super::layers::{Pixels, Slot};
use super::pipeline::Job;

/// One worker: take a job, draw it, hand it back, until there are none left.
pub(super) fn work(
    compositor: &mut CpuCompositor,
    slots: &[Slot],
    live: usize,
    jobs: &Mutex<Receiver<Job>>,
    done: &Sender<Result<Job, RenderError>>,
) {
    loop {
        let taken = {
            let jobs = jobs
                .lock()
                .expect("the job queue is only ever locked to take from");
            jobs.recv()
        };
        let Ok(mut job) = taken else {
            // The producer is finished, or has given up. Either way there is
            // nothing else coming.
            return;
        };
        let outcome = draw_frame(compositor, slots, live, &mut job);
        let sent = match outcome {
            Ok(()) => done.send(Ok(job)),
            Err(error) => done.send(Err(error.into())),
        };
        if sent.is_err() {
            return;
        }
    }
}

/// Composites one frame: every group into its canvas, innermost first, and
/// then everything on the frame itself.
///
/// Groups are drawn from the **last** slot to the first. A group's members —
/// and so its nested groups — always come after it ([`super::layers::open`]),
/// so walking backwards finishes every inner canvas before the group it is a
/// layer of is drawn, with no graph to walk and nothing drawn twice.
fn draw_frame(
    compositor: &mut CpuCompositor,
    slots: &[Slot],
    live: usize,
    job: &mut Job,
) -> Result<(), CompositeError> {
    let Job {
        properties,
        buffers,
        groups,
        canvas,
        ..
    } = job;
    // Type first: a revealing or counting title is set for this instant before
    // anything reads it, whether a group or the frame.
    for (slot, properties) in slots.iter().zip(properties.iter()) {
        if let Pixels::Typed { at, typing } = &slot.pixels {
            typing.draw(&mut buffers[live + at], properties.sweep, properties.number);
        }
    }
    for (index, slot) in slots.iter().enumerate().rev() {
        let Pixels::Composed { at } = slot.pixels else {
            continue;
        };
        // The canvas being written, and every canvas after it — which is where
        // any group inside this one has already been drawn.
        let (written, inner) = groups.split_at_mut(at + 1);
        let sources = Sources {
            buffers,
            live,
            groups: inner,
            first_group: at + 1,
        };
        let members = sources.layers(slots, properties, Some(index));
        compositor.offscreen(&mut written[at], &members)?;
    }
    let sources = Sources {
        buffers,
        live,
        groups,
        first_group: 0,
    };
    compositor.composite(canvas, &sources.layers(slots, properties, None))
}

/// Where each layer's pixels are, for one frame.
struct Sources<'b> {
    /// The decoded buffers, then the ones drawn afresh.
    buffers: &'b [Frame],
    /// How many of `buffers` are decoded, which is where the drawn ones start.
    live: usize,
    /// Group canvases, from `first_group` on.
    groups: &'b [Frame],
    /// Which group canvas `groups[0]` is.
    first_group: usize,
}

impl<'b> Sources<'b> {
    /// The layers drawn into `within` — a group, or the frame itself — in
    /// drawing order.
    fn layers(
        &self,
        slots: &'b [Slot],
        properties: &[Properties],
        within: Option<usize>,
    ) -> Vec<Layer<'b>> {
        slots
            .iter()
            .zip(properties)
            .filter(|(slot, _)| slot.within == within)
            .map(|(slot, properties)| Layer {
                source: match &slot.pixels {
                    Pixels::Held(pixels) => pixels,
                    Pixels::Live(at) => &self.buffers[*at],
                    Pixels::Drawn { at, .. } | Pixels::Typed { at, .. } => {
                        &self.buffers[self.live + *at]
                    }
                    Pixels::Composed { at } => &self.groups[*at - self.first_group],
                },
                properties: *properties,
                anchor: slot.anchor,
                origin: slot.origin,
            })
            .collect()
    }
}
