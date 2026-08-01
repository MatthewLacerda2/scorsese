//! Saying what a render contains, without rendering it.
//!
//! An agent that renders today renders blind: it gets a frame count, a
//! duration, and no way to answer *"is this the video I was asked for?"*
//! without a human opening the file. This module answers the half of that
//! question the document can answer — the right shot at the right second, the
//! fade where the fade was meant to be, the music not stopping two seconds
//! early — by reading a [`Plan`] and writing down what it already decided.
//!
//! It costs nothing. There is no ffmpeg here, no pixels and no bytes: the plan
//! has worked all of this out by the time it exists and currently throws it
//! away. So a description is available **before** a render as much as after
//! one, which makes it the cheapest review there is.
//!
//! **In seconds as well as frames.** The document is authored in frames on the
//! timeline grid, and a reader deciding whether a cut is right thinks in
//! seconds. Both are printed for every boundary rather than one being left as
//! an exercise, and [`Moment`] is the pair.
//!
//! What it structurally cannot catch is whether the pixels agree with the
//! document, because it is derived from the same document that might be wrong
//! about reality — a source that decoded black, a `native` clip whose media
//! turned out to be 4000px wide. That is [`crate::frames`]'s half of the job.
//!
//! [`Commentary`] is the one thing here that is read off the **document**
//! rather than off the plan: the script the project carries and the notes left
//! on its elements. What a cut contains includes why it is that way.

mod animation;
mod commentary;
mod cue;
mod display;
mod stretch;

use scorsese_core::{Fps, Frames};

use crate::plan::{Plan, Segment};

pub use animation::{Animated, Travel};
pub use commentary::Commentary;
pub use cue::{Cue, CueError};
pub use stretch::{Playing, Shown, Stretch};

/// A point on the timeline, said both ways.
///
/// The frame is what the document is authored in and what a keyframe is timed
/// against; the second is what anyone reading the description is thinking in.
/// Carrying both is the whole reason this type exists — a description that
/// printed one and made the reader convert would be a description nobody
/// checks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moment {
    /// The timeline frame. Boundaries are half-open, so a stretch's end is the
    /// frame *after* its last one — the frame the next stretch owns.
    pub frame: Frames,
    /// Where that frame falls in wall-clock seconds, on the timeline grid.
    pub seconds: f64,
}

impl Moment {
    /// A frame on `fps`'s grid, with its wall-clock time worked out once.
    pub fn at(frame: Frames, fps: Fps) -> Self {
        Self {
            frame,
            seconds: fps.seconds(frame),
        }
    }
}

/// What a render contains, stretch by stretch.
#[derive(Debug, Clone, PartialEq)]
pub struct Description {
    /// The grid the project was authored against, which every frame number
    /// here is counted on.
    pub timeline_fps: Fps,
    /// The grid the file is written on, which decides how many frames it has.
    pub out_fps: Fps,
    /// Where the render begins on the timeline — not frame 0 when a range was
    /// asked for.
    pub from: Moment,
    /// The frame just past the last one the render covers.
    pub to: Moment,
    /// How many frames the delivered file holds.
    pub out_frames: u64,
    /// Every stretch of timeline over which neither the picture nor the mix
    /// changes, in order and covering the whole range with no holes.
    pub stretches: Vec<Stretch>,
}

impl Description {
    /// Reads a plan back as a description of the cut it will produce.
    ///
    /// The stretches are cut at the union of the picture's boundaries and the
    /// mix's, so one line of the description is one stretch over which nothing
    /// at all changes — a shot that outlasts the narration under it is two
    /// stretches, because at the second one the answer to "what is audible"
    /// became different.
    pub fn of(plan: &Plan<'_>) -> Self {
        let fps = plan.timeline_fps();
        let cuts = boundaries(plan);
        let stretches = cuts
            .windows(2)
            .map(|pair| {
                let (from, to) = (pair[0], pair[1]);
                Stretch::between(
                    Moment::at(from, fps),
                    Moment::at(to, fps),
                    plan.out_frame_of(from),
                    covering(plan.segments(), from),
                    covering(plan.audio(), from),
                    fps,
                )
            })
            .collect();
        Self {
            timeline_fps: fps,
            out_fps: plan.out_fps(),
            from: Moment::at(plan.start(), fps),
            to: Moment::at(plan.end(), fps),
            out_frames: plan.out_frames(),
            stretches,
        }
    }

    /// How long the render runs, in wall-clock seconds.
    pub fn seconds(&self) -> f64 {
        self.to.seconds - self.from.seconds
    }

    /// True when nothing anywhere in the render makes a sound.
    pub fn is_silent(&self) -> bool {
        self.stretches
            .iter()
            .all(|stretch| stretch.sound.is_empty())
    }

    /// The frame of the delivered file each stretch begins on.
    ///
    /// The useful default for pulling stills out of a render: a boundary is
    /// where a cut can be one frame wrong, and it is the same reasoning the
    /// golden fixtures use when choosing which frames to compare. The end of
    /// the render is not among them — there is no frame there, only the edge.
    pub fn boundaries(&self) -> Vec<u64> {
        self.stretches
            .iter()
            .map(|stretch| stretch.at_frame)
            .collect()
    }

    /// The frame of the delivered file a wall-clock second lands on.
    ///
    /// Seconds are measured on the **timeline**, like everything else here, so
    /// a render of `--range 60:` and the project it came from agree about what
    /// "at 4 seconds" means. Anything past the end lands on the last frame
    /// rather than off it.
    pub fn out_frame_at(&self, seconds: f64) -> u64 {
        let offset = (seconds - self.from.seconds).max(0.0);
        let frame = (offset * self.out_fps.as_f64()).round().max(0.0) as u64;
        frame.min(self.out_frames.saturating_sub(1))
    }
}

/// Every frame at which either the picture or the mix can change, in order,
/// with the render's own end as the final one.
///
/// Both passes cover the range with no holes, so their starts plus the end are
/// the whole set — and the mix having no stretches at all (a silent film)
/// simply contributes nothing.
fn boundaries(plan: &Plan<'_>) -> Vec<Frames> {
    let mut cuts: Vec<Frames> = plan
        .segments()
        .iter()
        .chain(plan.audio())
        .map(|segment| segment.start)
        .chain([plan.end()])
        .collect();
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

/// The segment of a pass that covers `at`, if the pass has one.
fn covering<'a, 'p>(segments: &'a [Segment<'p>], at: Frames) -> Option<&'a Segment<'p>> {
    segments
        .iter()
        .find(|segment| segment.start <= at && at < segment.end())
}
