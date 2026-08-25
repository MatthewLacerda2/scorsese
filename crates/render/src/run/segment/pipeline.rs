//! Three stages, of which exactly one is parallel.
//!
//! **The producer decodes, in order, on one thread.** A decoder is a pipe out
//! of an ffmpeg process: frames arrive in sequence and only in sequence, so
//! reading them concurrently is not a thing that exists. It pulls one frame
//! from each layer that has a source — in lockstep, so every pipe drains evenly
//! and none of them blocks waiting for us — and emits a [`Job`].
//!
//! **The workers composite, concurrently.** Compositing a frame depends on
//! nothing but that frame's own inputs, so this is the stage that parallelises,
//! and it is the stage that was starved: ffmpeg already spreads decode and
//! encode across cores, while every transform, blend, grade and glyph in a
//! render happened on one.
//!
//! **The reorder stage writes, strictly in sequence.** Workers finish out of
//! order; a video file is a sequence. Finished frames are stashed by index and
//! handed to the encoder only when their turn comes, which is what keeps the
//! output of a parallel render bit-identical to a serial one.
//!
//! The producer and the reorder stage share the calling thread, because both
//! are sequential and neither is the bottleneck: while one blocks on a decode
//! the workers keep drawing, and the frames they finish are written the moment
//! the producer comes up for air.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};

use crate::content::elapsed;
use crate::error::RenderError;
use crate::pipe::Decoder;
use crate::plan::Segment;

use super::attach::End;
use super::layers::{Pixels, Slot};
use super::{Pass, Write};

/// The most frame buffers this stage may be holding at once, in bytes.
///
/// The development machine has 16 GB and is running an ffmpeg per layer beside
/// this, so a gigabyte is the most the compositing stage should ever be sitting
/// on. It is a ceiling and not a target: at 1080p the pipeline wants far less
/// and takes far less.
const BUDGET: usize = 1 << 30;

/// One output frame's worth of work, and the buffers it is done in.
///
/// A job owns everything a worker writes to, so nothing is shared but the
/// segment's held layers, which are read-only. It comes back to the producer
/// after the frame is written and is filled in again — allocating a canvas per
/// frame would be hundreds of megabytes a second of pure churn.
pub(super) struct Job {
    /// Which frame of the segment this is, which is what the reorder stage
    /// sorts on.
    index: u64,
    /// One instant of each layer, in the order they are drawn.
    properties: Vec<Properties>,
    /// One decoded frame per layer that has a source, in [`Pixels::Live`]
    /// order.
    buffers: Vec<Frame>,
    /// What the layers are composited onto, and what the encoder is given.
    canvas: Frame,
}

/// The buffers a render passes between its stages instead of allocating.
///
/// Kept for the whole render rather than per segment: a cut every two seconds
/// would otherwise re-allocate the entire pipeline's worth of frame buffers at
/// every cut, and they are megabytes each.
#[derive(Default)]
pub(super) struct Pools {
    free: Vec<Job>,
}

impl Pools {
    /// A job with buffers the right size for these decoders, reusing a spent
    /// one when there is one to reuse.
    ///
    /// Buffers a narrower segment does not need are kept rather than dropped:
    /// they are the ones the next wide segment would otherwise have to
    /// allocate, and a two-layer stretch between two four-layer ones is the
    /// ordinary shape of a cut.
    fn take(&mut self, decoders: &[Decoder], drawn: usize, resolution: Resolution) -> Job {
        let mut job = self.free.pop().unwrap_or_else(|| Job {
            index: 0,
            properties: Vec::new(),
            buffers: Vec::new(),
            canvas: Frame::black(resolution),
        });
        if job.canvas.resolution() != resolution {
            job.canvas = Frame::black(resolution);
        }
        // A source kept at its own size is not the size of the canvas, so the
        // buffer reading it is whatever its decoder produces — asked of the
        // decoder rather than worked out twice. Only a change of size costs an
        // allocation; a job reused within a segment costs none.
        for (buffer, decoder) in job.buffers.iter_mut().zip(decoders) {
            if buffer.resolution() != decoder.raster() {
                *buffer = Frame::black(decoder.raster());
            }
        }
        while let Some(decoder) = decoders.get(job.buffers.len()) {
            job.buffers.push(Frame::black(decoder.raster()));
        }
        // Then one raster-sized buffer per layer drawn afresh each frame. They
        // sit after the decoded ones in the same list, which is why a slot
        // carries its own index rather than deriving one: the two kinds are
        // counted separately and only the slot knows which it is.
        let wanted = decoders.len() + drawn;
        for buffer in job.buffers.iter_mut().skip(decoders.len()) {
            if buffer.resolution() != resolution {
                *buffer = Frame::black(resolution);
            }
        }
        while job.buffers.len() < wanted {
            job.buffers.push(Frame::black(resolution));
        }
        job
    }

    /// Takes a written frame's buffers back.
    fn recycle(&mut self, job: Job) {
        self.free.push(job);
    }
}

/// A stretch of timeline with nothing on it.
///
/// No layers, so compositing nothing is exactly what a gap looks like: the
/// cleared canvas, drawn once and written for as long as the gap lasts. There
/// is no pipeline here because there is no second frame to draw — every frame
/// of a gap is the same one.
pub(super) fn gap(
    compositor: &mut CpuCompositor,
    resolution: Resolution,
    frames: u64,
    pools: &mut Pools,
    write: Write<'_>,
) -> Result<(), RenderError> {
    let mut job = pools.take(&[], 0, resolution);
    compositor.composite(&mut job.canvas, &[])?;
    for _ in 0..frames {
        write(&job.canvas)?;
    }
    pools.recycle(job);
    Ok(())
}

/// Everything the three stages work on that the caller owns and reuses.
pub(super) struct Parts<'a> {
    /// What each layer contributes, in the order they are drawn.
    pub(super) slots: &'a [Slot],
    /// How many of those are redrawn every frame, which is how many raster-sized
    /// buffers a job needs beyond the decoded ones.
    pub(super) drawn: usize,
    /// One decoder per layer that has a source, in [`Pixels::Live`] order.
    pub(super) decoders: &'a mut [Decoder],
    /// One compositor per worker. They carry scratch buffers, which is why
    /// they are lent rather than made here.
    pub(super) compositors: &'a mut [CpuCompositor],
    /// Where spent frame buffers come back to.
    pub(super) pools: &'a mut Pools,
}

/// Runs `frames` frames of `segment` through the three stages, handing each
/// finished frame to `write` in order.
///
/// What comes back is how many frames each layer's source came up short by, in
/// [`Pixels::Live`] order — a fact about the media rather than a failure, and
/// the caller's to report.
pub(super) fn drive(
    pass: &Pass<'_>,
    segment: &Segment<'_>,
    frames: u64,
    parts: Parts<'_>,
    write: Write<'_>,
) -> Result<Vec<u64>, RenderError> {
    let Parts {
        slots,
        drawn,
        decoders,
        compositors,
        pools,
    } = parts;
    let capacity = capacity(
        compositors.len(),
        decoders.len() + drawn,
        pass.settings.resolution,
    );
    // How many of a job's buffers are decoded, which is where the drawn ones
    // start.
    let live = decoders.len();
    let mut missing = vec![0_u64; decoders.len()];

    let mut feed = Feed {
        slots,
        drawn,
        decoders,
        missing: &mut missing,
    };

    let (send_job, take_job) = channel::<Job>();
    // Locked around the receive rather than shared some cleverer way: a job is
    // a whole frame's compositing, so the hand-off is thousands of times
    // cheaper than the work it hands over.
    let take_job = Mutex::new(take_job);
    let (send_done, take_done) = channel::<Result<Job, RenderError>>();

    std::thread::scope(|scope| {
        for compositor in compositors.iter_mut() {
            let take_job = &take_job;
            let send_done = send_done.clone();
            scope.spawn(move || work(compositor, slots, live, take_job, &send_done));
        }
        // The only remaining sender is the workers': dropping this one is what
        // makes the results channel close when they are all done.
        drop(send_done);

        let mut produced = 0;
        let mut written = 0;
        let mut finished: BTreeMap<u64, Job> = BTreeMap::new();
        let outcome = (|| -> Result<(), RenderError> {
            while written < frames {
                // The producer blocks here — on its own next decode, or on
                // waiting below — rather than running ahead of the bound.
                while produced < frames && produced - written < capacity {
                    let job = produce(pass, segment, produced, &mut feed, pools)?;
                    produced += 1;
                    if send_job.send(job).is_err() {
                        break;
                    }
                }
                let done = take_done
                    .recv()
                    .expect("a worker holds the results channel until it is told to stop")?;
                finished.insert(done.index, done);
                while let Some(job) = finished.remove(&written) {
                    write(&job.canvas)?;
                    written += 1;
                    pools.recycle(job);
                }
            }
            Ok(())
        })();
        // Before the scope joins, and that is the point of saying it here: a
        // worker waits on this channel, so the threads only end once it is
        // gone — including on the way out of a failure.
        drop(send_job);
        outcome
    })?;

    Ok(missing)
}

/// Decodes one frame of every layer that has a source and works out what each
/// layer looks like at this instant.
fn produce(
    pass: &Pass<'_>,
    segment: &Segment<'_>,
    index: u64,
    feed: &mut Feed<'_>,
    pools: &mut Pools,
) -> Result<Job, RenderError> {
    let Feed {
        slots,
        drawn,
        decoders,
        missing,
    } = feed;
    let mut job = pools.take(decoders, *drawn, pass.settings.resolution);
    job.index = index;
    for (at, decoder) in decoders.iter_mut().enumerate() {
        if !decoder.read_into(&mut job.buffers[at])? {
            // This layer's source ran out before its clip did. Blank rather
            // than black: an upper layer that went opaque black would paint
            // over the tracks below it, which is not what running out of
            // footage means.
            job.buffers[at].fill_transparent();
            missing[at] += 1;
        }
    }
    // Keyframes are timed from each clip's own start, so that moving a clip
    // along the timeline never rewrites them. Which instant of the timeline
    // this output frame shows is the plan's to say.
    let at = pass.plan.timeline_frame_of(segment, index);
    job.properties.clear();
    job.properties.extend(
        segment
            .layers
            .iter()
            .map(|shot| Properties::at(shot.clip, elapsed(at, shot.clip))),
    );
    // Attached arrows last, because they read the properties every other layer
    // just resolved. This is the whole of the ordering the feature costs: one
    // pass over the layers, then the arrows that depend on them.
    draw_attached(slots, decoders.len(), &mut job, pass.settings.resolution);
    Ok(job)
}

/// Everything the producer fills a job *from*, which is only ever passed as a
/// set.
///
/// The buffer pool is deliberately not in here: the writing stage hands spent
/// jobs back to it in the same loop, so it stays a separate borrow rather than
/// one this holds for the whole scope.
struct Feed<'a> {
    /// What each layer contributes.
    slots: &'a [Slot],
    /// How many of them are redrawn every frame.
    drawn: usize,
    /// One per layer read from a source.
    decoders: &'a mut [Decoder],
    /// How many frames each source has come up short by so far.
    missing: &'a mut [u64],
}

/// Redraws every attached arrow for this frame, from wherever the clips they
/// follow have got to.
fn draw_attached(slots: &[Slot], live: usize, job: &mut Job, canvas: Resolution) {
    for slot in slots {
        let Pixels::Drawn { at, following } = &slot.pixels else {
            continue;
        };
        let ends = [&following.from, &following.to].map(|end| match end {
            End::Fixed(x, y) => (
                (x * f64::from(canvas.width())) as f32,
                (y * f64::from(canvas.height())) as f32,
            ),
            End::Follows { layer, side } => {
                slots[*layer]
                    .rect
                    .point_at(*side, &job.properties[*layer], canvas)
            }
        });
        crate::shape::paint_arrow(
            &mut job.buffers[live + at],
            &following.shape,
            ends[0],
            ends[1],
        );
    }
}

/// One worker: take a job, draw it, hand it back, until there are none left.
fn work(
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
        let layers: Vec<Layer<'_>> = slots
            .iter()
            .zip(&job.properties)
            .map(|(slot, properties)| Layer {
                source: match &slot.pixels {
                    Pixels::Held(pixels) => pixels,
                    Pixels::Live(at) => &job.buffers[*at],
                    Pixels::Drawn { at, .. } => &job.buffers[live + *at],
                },
                properties: *properties,
                anchor: slot.anchor,
                origin: slot.origin,
            })
            .collect();
        let outcome = compositor.composite(&mut job.canvas, &layers);
        let sent = match outcome {
            Ok(()) => done.send(Ok(job)),
            Err(error) => done.send(Err(error.into())),
        };
        if sent.is_err() {
            return;
        }
    }
}

/// How many frames may be in flight — produced but not yet written.
///
/// Two numbers decide it, and the smaller wins.
///
/// **What the pipeline needs** is `workers + 2`: every worker holding a job,
/// one queued so that a worker finishing does not wait on a decode, and one
/// being filled by the producer. Deeper buys nothing, because a single
/// sequential decode is what feeds this and throughput is bounded by whichever
/// of decode and composite is slower — a longer queue only absorbs jitter.
///
/// **What the machine has** is the other, and it is why this is a number
/// chosen up front rather than discovered by swapping. A job holds one decoded
/// buffer per layer with a source plus the canvas it is drawn onto, so the
/// stage can be sitting on `capacity × (layers + 1)` frames: 8.3 MB each at
/// 1080p, 33.2 MB at 4K. Seven workers on a four-layer 4K render would be
/// 1.5 GB of frame buffers under [`BUDGET`]'s gigabyte, so the depth gives way
/// and the pipeline runs shallower rather than the render going to swap.
///
/// Two is the floor: one frame being composited while the next is decoded is
/// the least that is still a pipeline.
fn capacity(workers: usize, live: usize, resolution: Resolution) -> u64 {
    let frame = resolution.width() as usize * resolution.height() as usize * BYTES_PER_PIXEL;
    let job = frame.saturating_mul(live + 1).max(1);
    let wanted = (workers + 2).min(BUDGET / job).max(2);
    wanted as u64
}
