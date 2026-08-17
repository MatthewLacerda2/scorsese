//! Where each clip's content lands on the frame, at one instant, without
//! rendering it.
//!
//! The compositor has always known this — it is what an arrow attaching to a
//! box reads — and nothing outside the render could ask. Authoring a layout was
//! therefore a guessing loop: predict how tall a block of text will be, render
//! a frame, look at it, adjust. That loop costs an agent far more than a
//! person, because it has to composite a still and *look* at it once per
//! instant it cares about. This is the door onto the number that already
//! exists.
//!
//! **In fractions of the raster, never pixels.** A rectangle is only useful
//! here for editing the document, and the document is written in fractions of
//! the frame — a `transform.position`, a shape's `width`, a text `size`. An
//! answer in pixels would be an answer in a unit nothing can be written back
//! in, and it would change with the render's resolution.
//!
//! **It reports; it never moves anything.** Knowing where a thing is does not
//! make the editor the one to centre it, and automatic layout is a different
//! program.
//!
//! It costs nothing: no ffmpeg, no decode, no money. The one thing it will not
//! do is spawn a probe to find out how big an unmeasured source is — that is a
//! price worth paying to produce a video and not to answer a question about
//! one — so such a clip is reported as one it cannot answer for, by name and
//! with the reason.

mod display;

use std::path::Path;

use scorsese_compositor::{Area, Properties, Resolution};
use scorsese_core::{AssetKind, Clip, Frames, Project, TrackId};

use crate::content::{self, Content, Rect};
use crate::describe::Moment;
use crate::error::RenderError;
use crate::plan::{FrameRange, Plan, Shot};
use crate::raster::Sizes;
use crate::text::Painter;

/// Where everything on one frame of the timeline sits.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The instant this is about, in both units the timeline has.
    pub at: Moment,
    /// The raster it was worked out against. The answer is in fractions, but
    /// the frame's *shape* still decides it: a `fit` picture letterboxes
    /// against this aspect, and a block of text wraps against this width.
    pub raster: Resolution,
    /// Every clip on screen whose content has a rectangle, **bottom layer
    /// first** — the order they are drawn in, so the last one listed is the one
    /// on top.
    pub placed: Vec<Placement>,
    /// Every clip this could not answer for, and why. Said rather than left
    /// out: "not on screen at this instant" and "on screen and immeasurable"
    /// are different answers, and a caller that asked about a clip deserves to
    /// know which it got.
    pub unplaced: Vec<Unplaced>,
}

impl Layout {
    /// Where every clip's content lands at timeline frame `at`.
    ///
    /// `raster` is the frame it is worked out against — the resolution a render
    /// would be delivered at. Sequences the timeline exactly as a render does
    /// and reads the compositor's own matrix for each layer, so this cannot
    /// disagree with the picture.
    pub fn at(
        project: &Project,
        project_root: &Path,
        raster: Resolution,
        at: Frames,
    ) -> Result<Self, RenderError> {
        let fps = project.timeline_fps;
        // One frame of plan, exactly as a still is composited from: the cuts a
        // plan splits on are boundaries strictly inside its range, and a single
        // frame has no inside for one to fall in.
        let plan = Plan::build(project, fps, FrameRange::just(at))?;
        let segment = plan
            .segments()
            .first()
            .expect("a plan over one frame has that frame's segment in it");
        let sizes = Sizes::recorded(&plan, project_root);
        let mut painter = Painter::default();

        let mut placed = Vec::new();
        let mut unplaced = Vec::new();
        for shot in &segment.layers {
            match rectangle(shot, &mut painter, raster, project_root, &sizes) {
                Ok(rect) => {
                    // The layer's own transform at this instant, resolved the
                    // way the renderer resolves it, then put through the
                    // compositor's matrix rather than a copy of it.
                    let properties = Properties::at(shot.clip, content::elapsed(at, shot.clip));
                    placed.push(Placement {
                        clip: shot.clip.id.to_string(),
                        track: shot.track.to_string(),
                        asset: shot.asset.id.to_string(),
                        kind: shot.asset.kind,
                        area: Region::of(rect.bounds_on(&properties, raster), raster),
                    });
                }
                Err(why) => unplaced.push(Unplaced::new(shot.track, shot.clip, why)),
            }
        }

        let audible = plan
            .audio()
            .first()
            .map_or(&[][..], |segment| &segment.layers);
        for (track, clip) in project.clips() {
            if segment.layers.iter().any(|shot| shot.clip.id == clip.id) {
                continue;
            }
            let sounding = audible.iter().any(|shot| shot.clip.id == clip.id);
            let why = if sounding {
                Absence::Sound
            } else {
                Absence::OffScreen
            };
            unplaced.push(Unplaced::new(&track.id, clip, why));
        }

        Ok(Self {
            at: Moment::at(at, fps),
            raster,
            placed,
            unplaced,
        })
    }

    /// The rectangle a named clip's content covers here, if it has one.
    pub fn of(&self, clip: &str) -> Option<Region> {
        self.placed
            .iter()
            .find(|placement| placement.clip == clip)
            .map(|placement| placement.area)
    }
}

/// One clip's content, and where it lands.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// The clip, by the id every note and validation error names it by.
    pub clip: String,
    /// The track it sits on, which is what decides what is in front of what.
    pub track: String,
    /// The asset it shows, by id.
    pub asset: String,
    /// What kind of thing that asset is — half of why a rectangle is where it
    /// is, and the difference between a title and the panel behind it.
    pub kind: AssetKind,
    /// What the content covers, in fractions of the raster.
    pub area: Region,
}

/// A clip with no rectangle at this instant, and why it has none.
#[derive(Debug, Clone, PartialEq)]
pub struct Unplaced {
    /// The clip, by the id every note and validation error names it by.
    pub clip: String,
    /// The track it sits on.
    pub track: String,
    /// Why there is no answer for it.
    pub why: Absence,
}

impl Unplaced {
    fn new(track: &TrackId, clip: &Clip, why: Absence) -> Self {
        Self {
            clip: clip.id.to_string(),
            track: track.to_string(),
            why,
        }
    }
}

/// The reasons a clip has no rectangle. Three genuinely different answers, and
/// collapsing them into "no" is what makes an answer useless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Absence {
    /// It is not on screen at this instant. It has a rectangle at instants it
    /// *is* on screen — ask again there.
    OffScreen,
    /// It is in the mix here and puts nothing on the picture. A sketch a
    /// narration prompt has not been generated from does show a card, and that
    /// one is placed like anything else.
    Sound,
    /// It is on screen and its rectangle could not be worked out. The sentence
    /// says why, because each of these is a different thing to do about it.
    Unknown(String),
}

/// A rectangle of the frame, in fractions of the raster: `0.0` its left or top
/// edge and `1.0` its right or bottom.
///
/// The unit the document is written in, so a number read out of one of these
/// can be written straight back into a `transform.position` or a shape's
/// `width`. A rectangle can legitimately fall outside `0..1` — a layer moved
/// off the edge of the frame is half off it — and saying so is more use than
/// clamping it into a lie.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    /// Fraction of the frame's width from its left edge to this rectangle's.
    pub left: f64,
    /// Fraction of the frame's height from its top edge to this rectangle's.
    pub top: f64,
    /// How wide, as a fraction of the frame's width.
    pub width: f64,
    /// How tall, as a fraction of the frame's height.
    pub height: f64,
}

impl Region {
    /// The rectangle's right edge.
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    /// The rectangle's bottom edge.
    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }

    /// True when the two rectangles share any of the frame at all.
    ///
    /// Touching edges do not overlap: a caption resting exactly on the top of
    /// the panel below it is a layout somebody meant, and calling it a
    /// collision would make the answer noise.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.left < other.right()
            && other.left < self.right()
            && self.top < other.bottom()
            && other.top < self.bottom()
    }

    /// Pixels of a raster as fractions of it.
    fn of(area: Area, raster: Resolution) -> Self {
        let across = f64::from(raster.width());
        let down = f64::from(raster.height());
        Self {
            left: f64::from(area.left) / across,
            top: f64::from(area.top) / down,
            width: f64::from(area.width) / across,
            height: f64::from(area.height) / down,
        }
    }
}

/// Where one shot's content sits within its own layer raster, or the sentence
/// saying why that is not answerable.
///
/// The decision itself is [`crate::content::within`], which is what the render
/// draws from — this only turns the two cases it cannot resolve into words.
fn rectangle(
    shot: &Shot<'_>,
    painter: &mut Painter,
    raster: Resolution,
    project_root: &Path,
    sizes: &Sizes,
) -> Result<Rect, Absence> {
    let rest = |source, area| Rect {
        source,
        area,
        anchor: shot.clip.anchor,
    };
    let content = content::within(shot, painter, raster, project_root)
        // A font that will not open, or media that has gone missing. Both are
        // faults `scorsese check` reports in full; here they are the reason
        // this one clip has no answer.
        .map_err(|error| Absence::Unknown(error.to_string()))?;
    match content {
        Content::Drawn(area) => Ok(rest(raster, area)),
        Content::Unbounded => Err(Absence::Unknown(
            "an arrow is a line between two points and has no box of its own".to_owned(),
        )),
        Content::Media if sizes.is_unmeasured(shot) => Err(Absence::Unknown(format!(
            "its source's own pixel size is not recorded, and a `{}` clip's rectangle \
             depends on it — `scorsese probe` writes it into the document",
            crate::describe::display::fit(shot.clip.fit)
        ))),
        // A decoded picture fills the raster its frames arrive at, which for a
        // letterboxed `fit` is the picture and not the canvas.
        Content::Media => {
            let source = sizes.fitting(shot, raster).raster_at(raster);
            Ok(rest(source, Area::whole(source)))
        }
    }
}
