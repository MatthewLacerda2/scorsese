//! Two layers drawn on top of each other, which nothing in the document says.
//!
//! Every layer is placed independently, in fractions, against a frame it does
//! not otherwise know about — so two of them colliding is not a field anybody
//! can read. It is an emergent fact about numbers that were each individually
//! reasonable, and until now the only way to find it was to composite a still
//! and look at it, which finds the collisions at the instants somebody happened
//! to sample and says nothing about the rest.
//!
//! **Always a warning.** Layers overlapping on purpose is ordinary and
//! constant: a title over a shot, a scrim over a picture, a label deliberately
//! inside its box. This can therefore never fail a render or move an exit code.
//!
//! **Which is why most of this file is the filter.** A warning that always
//! fires is a warning nobody reads, and overlap fires constantly unless the
//! deliberate arrangements are subtracted first. Every threshold below is a
//! judgement about which shapes are authored on purpose, and each one is
//! written down with why it is the number it is. Where the choice was close the
//! filter goes quiet: a collision this misses is a still away from being found,
//! and a report nobody trusts is not.
//!
//! The thresholds were then held against a real project — a 290-clip
//! seven-minute explainer, all panels and labels — where a correct filter says
//! nothing at all, and against the same project with one caption dropped onto
//! the one below it, where it has to say exactly that. That pair of runs is what
//! moved the text numbers to where they are, and it is the experiment to repeat
//! before changing one.
//!
//! The rectangles are [`Layout`]'s, never a copy of them — the same numbers the
//! compositor's own matrix produces for the frame it draws.

use std::collections::HashSet;
use std::path::Path;

use scorsese_compositor::Resolution;
use scorsese_core::{AssetKind, Fps, Frames, Project, TrackKind};

use crate::layout::{Layout, Placement, Region};
use crate::plan::{FrameRange, Plan, Segment};

/// A layer covering this much of the frame is a background or a scrim, and
/// overlaps everything above it by construction.
///
/// Area rather than either dimension, so that a letterboxed picture is judged
/// by how much of the frame it actually fills. At 0.85 a rectangle would have
/// to sit within about 4% of every edge to escape — which is not a panel with a
/// margin, it is the frame.
const FRAME_FILLING: f64 = 0.85;

/// How close in size two rectangles must be before their overlap is worth a
/// word: the smaller at least this fraction of the larger's area.
///
/// This is the threshold that does most of the suppressing, and it is what
/// makes a title over a shot silent. The deliberate arrangements — a caption on
/// a picture, a label on a diagram, a lower third under everything — are all an
/// order of magnitude apart in size. Two layers that were each meant to occupy
/// their own part of the frame are, by construction, of the same order. A
/// quarter of the area is a factor of two in each dimension, which is where
/// "one is decoration on the other" stops being the obvious reading.
const COMPARABLE: f64 = 0.25;

/// How much of the smaller rectangle the overlap must cover, for a pair of
/// layers whose rectangles are the shapes they draw.
///
/// A rectangle here is an upright bounding box: a turned layer is boxed with the
/// corners it turned out of, and a letterboxed picture with the bars. So boxes
/// that graze each other by a few percent are often not overlapping at all in
/// the picture. A fifth is where the two are unarguably on top of one another.
const MINIMUM_BITE: f64 = 0.2;

/// Overlaps shorter than this many seconds are not reported.
///
/// The span is what separates a bug from a transition. `scorsese dissolve`
/// writes a crossover of half a second by default and — because two clips on
/// one track may not overlap — moves the incoming clip to a track above the
/// outgoing one, which is exactly the shape this check looks for. A second is
/// twice that, so a slow dissolve stays quiet too, while a real collision lasts
/// as long as both clips are on screen: seconds at least, and usually the whole
/// of a shot.
const BRIEFEST: f64 = 1.0;

/// The window a bite has to fall in when one of the two layers is **text**,
/// which is a narrower window than anything else gets, for one reason.
///
/// A text layer's rectangle is deliberately its **wrap block** — `max_width`
/// wide — and not the longest line in it. That is settled compositor behaviour
/// (it is what keeps a column's left edge still as the wording changes, and what
/// an arrow attaches to), and asking for a tighter box here would be a second
/// way to compute a rectangle. So the box is generous by design: a label reading
/// `T` in a `max_width` of 0.048 has a box eight times the width of the letter,
/// and two centred captions side by side can share a quarter of their boxes with
/// clear air between the words.
///
/// Hence both ends:
///
/// - **Half, at the bottom.** At half, the centre of the smaller block is inside
///   the other layer — and a block sets its glyphs about that centre. Below it,
///   what the two share can easily be nothing but the margin the wrap box added.
/// - **0.85 at the top.** Nearly all of one inside the other is the
///   label-in-a-box arrangement this format encourages most, whether or not the
///   box pokes a pixel out of the panel behind it.
///
/// A pair with no text in it gets no ceiling: one picture wholly hiding another
/// the same size is not a label, it is a layer nobody can see.
const TEXT_WINDOW: std::ops::Range<f64> = 0.5..0.85;

/// The frame every rectangle here is worked out against.
///
/// A project carries no resolution — that is chosen per render — so a question
/// about where a layer lands has to be asked against some frame, and 16:9 at
/// 1080p is what the rest of the tool defaults to. It decides two things: what a
/// `fit` picture letterboxes into, and how wide a block of text wraps. Neither
/// is sensitive enough to a raster for a different one to change what this
/// reports, and an overlap that only exists at one delivery size is not one this
/// check is for.
fn reference() -> Resolution {
    Resolution::new(1920, 1080).expect("1920x1080 is a legal raster")
}

/// Every pair of layers that spends long enough drawn over each other to be
/// worth a word, already worded.
///
/// Walks the instants where the visible set changes — the boundaries a [`Plan`]
/// already cuts on, never every frame — and asks [`Layout`] where things landed
/// at each. No ffmpeg, no decode, and nothing is probed: a segment whose source
/// size was never recorded is one [`Layout`] declines to place, and this is
/// silent about it rather than guessing.
///
/// A project that cannot be sequenced at all has nothing to say here. That is
/// not this function swallowing a fault — an empty timeline is the checkup's to
/// report, through validation, in a sentence that names the real problem.
pub(super) fn collisions(project: &Project, project_dir: &Path) -> Vec<String> {
    let fps = project.timeline_fps;
    let Ok(plan) = Plan::build(project, fps, FrameRange::ALL) else {
        return Vec::new();
    };
    let picture: HashSet<&str> = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Video)
        .map(|track| track.id.as_str())
        .collect();

    let mut runs: Vec<Run> = Vec::new();
    for segment in plan.segments() {
        if !stacked(segment, &picture) {
            continue;
        }
        // A layout this instant cannot be worked out for is not a hole in the
        // answer worth complaining about twice: whatever made it unanswerable —
        // a font that will not open, media that has gone — is already a finding
        // of its own in the same report.
        let Ok(layout) = Layout::at(project, project_dir, reference(), segment.start) else {
            continue;
        };
        let layers: Vec<&Placement> = layout
            .placed
            .iter()
            .filter(|placement| picture.contains(placement.track.as_str()))
            .collect();
        for (above, over) in layers.iter().enumerate() {
            // Bottom layer first, so everything before `above` is under it.
            // Same track is skipped rather than trusted to be impossible: two
            // clips on one track cannot overlap in time, and this check is not
            // the place to depend on that.
            for under in &layers[..above] {
                if under.track == over.track {
                    continue;
                }
                if let Some(bite) = collision(under, over) {
                    record(&mut runs, under, over, segment, bite);
                }
            }
        }
    }

    runs.iter()
        .filter(|run| run.seconds(fps) >= BRIEFEST)
        .map(|run| run.says(fps))
        .collect()
}

/// Whether this stretch has layers on two different picture tracks at all.
///
/// The cheap question asked before the expensive one: most segments of most
/// edits are one shot, and working out a whole layout to discover there was
/// nothing to compare is the cost this check would otherwise pay per cut.
fn stacked(segment: &Segment<'_>, picture: &HashSet<&str>) -> bool {
    let mut tracks = segment
        .layers
        .iter()
        .map(|shot| shot.track.as_str())
        .filter(|track| picture.contains(track));
    let first = tracks.next();
    tracks.any(|track| Some(track) != first)
}

/// How much of the smaller layer the overlap swallows, when the pair is one
/// worth reporting at all.
///
/// `None` is the answer for every deliberate arrangement, and the order of the
/// tests is the order of how often they fire.
fn collision(under: &Placement, over: &Placement) -> Option<Bite> {
    let (below, above) = (visible(under.area)?, visible(over.area)?);
    let shared = overlap(below, above)?;
    let (small, smaller, larger) = if area(below) <= area(above) {
        (under, below, above)
    } else {
        (over, above, below)
    };
    if area(larger) >= FRAME_FILLING {
        return None;
    }
    if area(smaller) < COMPARABLE * area(larger) {
        return None;
    }
    let bite = area(shared) / area(smaller);
    if !window(under, over).contains(&bite) {
        return None;
    }
    Some(Bite {
        fraction: bite,
        smaller: small.clip.clone(),
    })
}

/// How deep the overlap has to be, and how deep it may be, for this pair.
///
/// [`TEXT_WINDOW`] whenever either layer is text, and the reason it is narrower
/// is written down there. Everything else gets a floor and no ceiling —
/// `f64::INFINITY`, so that one layer entirely under another is still inside the
/// window rather than falling off the end of it.
fn window(under: &Placement, over: &Placement) -> std::ops::Range<f64> {
    if under.kind == AssetKind::Text || over.kind == AssetKind::Text {
        TEXT_WINDOW
    } else {
        MINIMUM_BITE..f64::INFINITY
    }
}

/// The part of a rectangle that is on the frame at all, if any of it is.
///
/// A layer moved off the edge is honestly reported as off it, and half of one
/// hanging over the side is drawn only where the frame is — so the part that
/// was never drawn cannot collide with anything.
fn visible(region: Region) -> Option<Region> {
    overlap(
        region,
        Region {
            left: 0.0,
            top: 0.0,
            width: 1.0,
            height: 1.0,
        },
    )
}

/// The rectangle two rectangles share, when they share one with area in it.
fn overlap(one: Region, two: Region) -> Option<Region> {
    if !one.overlaps(&two) {
        return None;
    }
    let left = one.left.max(two.left);
    let top = one.top.max(two.top);
    let region = Region {
        left,
        top,
        width: one.right().min(two.right()) - left,
        height: one.bottom().min(two.bottom()) - top,
    };
    (area(region) > 0.0).then_some(region)
}

/// How much of the frame a rectangle covers, as a fraction of its area.
fn area(region: Region) -> f64 {
    (region.width * region.height).max(0.0)
}

/// One instant's answer about one pair: how deep the overlap is, and which
/// layer that is a fraction of.
struct Bite {
    fraction: f64,
    smaller: String,
}

/// One pair of clips, over the stretch of timeline they are drawn over each
/// other for.
///
/// A run rather than an instant, because the span is the whole of the
/// judgement: two frames of overlap is a transition and nine seconds of it is
/// the bug, and only a report that has walked both boundaries can tell them
/// apart.
struct Run {
    under: String,
    over: String,
    from: Frames,
    to: Frames,
    /// The worst instant of the span, and the layer it was worst for. The
    /// deepest reading rather than the first: an overlap that grows as a layer
    /// animates into place is worth reporting at its full depth.
    worst: Bite,
}

impl Run {
    fn seconds(&self, fps: Fps) -> f64 {
        fps.seconds(self.to) - fps.seconds(self.from)
    }

    /// The finding, worded. Both units, like every other span this crate
    /// prints, and the layer on top is named because it is the one hiding the
    /// other.
    fn says(&self, fps: Fps) -> String {
        format!(
            "clips `{under}` and `{over}` are drawn over each other from \
             {from:.2}–{to:.2}s (f{first}–{last}), `{over}` on top — the overlap \
             covers {bite:.0}% of `{smaller}`",
            under = self.under,
            over = self.over,
            from = fps.seconds(self.from),
            to = fps.seconds(self.to),
            first = self.from.get(),
            last = self.to.get(),
            bite = self.worst.fraction * 100.0,
            smaller = self.worst.smaller,
        )
    }
}

/// Adds one instant's collision to the run it continues, or starts a new one.
///
/// A pair is one run for as long as it is uninterrupted: the segments a plan
/// cuts on abut exactly, so a run continues when the next stretch begins where
/// the last one ended, and a pair that collides, separates and collides again
/// is honestly two findings.
fn record(
    runs: &mut Vec<Run>,
    under: &Placement,
    over: &Placement,
    segment: &Segment<'_>,
    bite: Bite,
) {
    if let Some(run) = runs
        .iter_mut()
        .rev()
        .find(|run| run.under == under.clip && run.over == over.clip)
        && run.to == segment.start
    {
        run.to = segment.end();
        if bite.fraction > run.worst.fraction {
            run.worst = bite;
        }
        return;
    }
    runs.push(Run {
        under: under.clip.clone(),
        over: over.clip.clone(),
        from: segment.start,
        to: segment.end(),
        worst: bite,
    });
}
