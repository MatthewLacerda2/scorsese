//! Clips that travel along an arrow: where on the frame `follow.progress` puts
//! them, one instant at a time.
//!
//! **Two callers, and they must never disagree** — the rule [`crate::content`]
//! applies to rectangles, applied to a follower. The render places every
//! follower here before it redraws a frame; [`crate::layout`] places them here
//! before it reports where anything landed, so `describe --at` and `check`'s
//! overlap pass see the packet where it is drawn and not where its transform
//! alone would put it. Neither asks in terms of the other's types: a frame is a
//! list of [`Layer`]s and a matching list of resolved properties.
//!
//! **The second place one layer's geometry depends on another's**, after an
//! arrow attached to a box ([`crate::attach`]), and it is paid for the same
//! way: the arrow is resolved to a layer of this segment when it is on screen,
//! so no ordering is introduced beyond "place the followers, then redraw". Core
//! refuses a path that depends on another path — an arrow a clip follows may
//! not be attached to a follower — which is what keeps that one pass enough.
//!
//! **The arrow need not be on screen.** Its line is geometry, and a dot setting
//! off before the connector has drawn itself in is an ordinary thing to want,
//! so an arrow between two fixed places is followed from the document when it
//! is not drawn here. An arrow with an attached end is the exception: where it
//! runs is a fact about the layers on screen, and without them there is no
//! line — the follower is left out and the render says so, as an attached
//! arrow with nothing to point at is ([`crate::report::Note::FollowLost`]).
//!
//! **What moves is the layer's transform, nothing else.** The path gives the
//! point of the clip's content its `origin` names — the dot's middle, by
//! default — and `transform.position` is added on top as an offset; `orient`
//! adds the line's heading to `transform.rotation`. Everything after that is
//! the ordinary compositing path, so a follower fades, scales and blurs exactly
//! as it would standing still.

use scorsese_compositor::shape::{Outline, Station, measure};
use scorsese_compositor::{Properties, Resolution, on_canvas};
use scorsese_core::{
    Anchor, AnchorX, AnchorY, Clip, ClipId, Frames, Geometry, Origin, Project, Shape,
};

use crate::attach::{self, Following};
use crate::content::{Rect, elapsed};

/// What placing a follower needs to know about one layer of the frame.
pub(crate) struct Layer<'a> {
    /// The layer's clip.
    pub(crate) clip: &'a Clip,
    /// Its shape, when it is one — an arrow being the shape that matters here.
    pub(crate) shape: Option<&'a Shape>,
    /// The group it is drawn into, by its index in the same list.
    pub(crate) within: Option<usize>,
    /// Where its content sits within its own raster — for an arrow, the whole
    /// raster, which is what it is drawn at.
    pub(crate) rect: Rect,
}

/// One clip that travels along an arrow, and where that arrow is.
pub(crate) struct Rider {
    /// Which layer travels.
    pub(crate) layer: usize,
    /// Where the line it travels along comes from.
    path: Path,
    /// Whether it turns to face along the line.
    orient: bool,
}

impl Rider {
    /// The layers this one's arrow is attached to, whose rectangles decide
    /// where the line runs and so where this layer lands.
    pub(crate) fn leans_on(&self) -> Vec<usize> {
        match &self.path {
            Path::Layer {
                ends: Some(ends), ..
            } => ends.layers().collect(),
            Path::Layer { ends: None, .. } | Path::Absent { .. } | Path::Lost => Vec::new(),
        }
    }
}

/// Where a follower's line is, once the segment is known.
enum Path {
    /// The arrow is a layer of this segment, at this index — and, when it is
    /// attached, where its ends come from.
    Layer {
        /// The arrow's layer.
        arrow: usize,
        /// Its attached ends, resolved against this segment's layers.
        ends: Option<Following>,
    },
    /// The arrow is not on screen here and runs between two fixed places, so
    /// its line is read off the document: its clip, for the transform it would
    /// have at this instant, and its shape, for the line itself.
    Absent {
        /// The arrow's clip, boxed because a clip is large and this variant
        /// is the rare one.
        clip: Box<Clip>,
        /// The arrow's shape.
        shape: Shape,
    },
    /// There is no line to follow here — an attached arrow off screen, or on
    /// screen with nothing to point at — so the follower is not drawn.
    Lost,
}

/// Every follower among `layers`, and the arrow each one travels along.
///
/// A follower whose arrow is lost comes back with the arrow's clip id, so the
/// caller can say so; `project` is where an arrow that is not on screen is
/// looked for. Validation has already refused an arrow that is missing, is not
/// an arrow, or sits across a group's edge, so a lookup that fails here is a
/// document that did not validate, and its follower is simply left alone.
pub(crate) fn riders(
    layers: &[Layer<'_>],
    project: &Project,
) -> (Vec<Rider>, Vec<(String, String)>) {
    let mut riders = Vec::new();
    let mut lost = Vec::new();
    for (layer, entry) in layers.iter().enumerate() {
        let Some(follow) = &entry.clip.follow else {
            continue;
        };
        let path = match layer_of(layers, entry.within, &follow.clip) {
            Some(arrow) => on_screen_path(arrow, layers),
            None => absent_path(project, &follow.clip),
        };
        let Some(path) = path else {
            continue;
        };
        if matches!(path, Path::Lost) {
            lost.push((entry.clip.id.to_string(), follow.clip.to_string()));
        }
        riders.push(Rider {
            layer,
            path,
            orient: follow.orient,
        });
    }
    (riders, lost)
}

/// Which of `layers` drawn into `within` is clip `id`, if it is on screen.
fn layer_of(layers: &[Layer<'_>], within: Option<usize>, id: &ClipId) -> Option<usize> {
    layers
        .iter()
        .position(|layer| layer.within == within && &layer.clip.id == id)
}

/// The line of an arrow drawn in this segment: followed wherever it is drawn,
/// and lost when it is an attached arrow drawing nothing for want of a target.
fn on_screen_path(arrow: usize, layers: &[Layer<'_>]) -> Option<Path> {
    let shape = layers[arrow].shape?;
    if !matches!(shape.geometry, Geometry::Arrow { .. }) {
        return None;
    }
    if !attach::is_attached(shape) {
        return Some(Path::Layer { arrow, ends: None });
    }
    let within = layers[arrow].within;
    Some(
        match attach::following(shape, |id| layer_of(layers, within, id)) {
            Some(ends) => Path::Layer {
                arrow,
                ends: Some(ends),
            },
            None => Path::Lost,
        },
    )
}

/// The line of an arrow that is not on screen here, read off the document.
fn absent_path(project: &Project, id: &ClipId) -> Option<Path> {
    let (_, clip) = project.every_clip().find(|(_, clip)| &clip.id == id)?;
    let shape = project.asset(&clip.asset)?.shape.as_ref()?;
    if !matches!(shape.geometry, Geometry::Arrow { .. }) {
        return None;
    }
    Some(if attach::is_attached(shape) {
        Path::Lost
    } else {
        Path::Absent {
            clip: Box::new(clip.clone()),
            shape: shape.clone(),
        }
    })
}

/// Moves every follower to its place on its arrow at this instant, and
/// returns the layers that have none — which are made fully transparent.
///
/// `properties` is the whole frame's, freshly resolved, in the order of
/// `layers`; `local` is the instant on each layer's own timeline, which is also
/// its arrow's, since the two share one. The render runs this before any
/// per-frame layer is redrawn, so an arrow attached to a follower meets it
/// where it has got to.
pub(crate) fn place(
    riders: &[Rider],
    layers: &[Layer<'_>],
    properties: &mut [Properties],
    local: impl Fn(usize) -> Frames,
    canvas: Resolution,
) -> Vec<usize> {
    let mut stranded = Vec::new();
    for rider in riders {
        let progress = properties[rider.layer].progress;
        let station = match &rider.path {
            Path::Layer { arrow, ends } => {
                let rect = layers[*arrow].rect;
                let line = line_on_screen(*arrow, ends.as_ref(), layers, properties, canvas);
                let lay = Laid {
                    properties: &properties[*arrow],
                    anchor: rect.anchor,
                    origin: rect.origin,
                    source: rect.source,
                };
                line.and_then(|line| lay.station(line, progress, canvas))
            }
            Path::Absent { clip, shape } => {
                let arrow = Properties::at(clip, elapsed(local(rider.layer), clip));
                let lay = Laid {
                    properties: &arrow,
                    anchor: clip.anchor,
                    origin: clip.origin,
                    source: canvas,
                };
                let line = crate::shape::outline(&shape.geometry, canvas, clip.anchor);
                lay.station(line, progress, canvas)
            }
            Path::Lost => None,
        };
        let follower = &mut properties[rider.layer];
        match station {
            Some(station) => ride(
                follower,
                layers[rider.layer].rect,
                station,
                rider.orient,
                canvas,
            ),
            // No line, so no place: a clip parked in the middle of the frame
            // would be a worse answer than one that is plainly absent.
            None => {
                follower.opacity = 0.0;
                stranded.push(rider.layer);
            }
        }
    }
    stranded
}

/// An arrow on screen as it runs at this instant, in its own raster: from
/// wherever its attached ends have got to, or from the document.
fn line_on_screen(
    arrow: usize,
    ends: Option<&Following>,
    layers: &[Layer<'_>],
    properties: &[Properties],
    canvas: Resolution,
) -> Option<Outline> {
    let shape = layers[arrow].shape?;
    let Some(following) = ends else {
        let anchor = layers[arrow].rect.anchor;
        return Some(crate::shape::outline(&shape.geometry, canvas, anchor));
    };
    let [from, to] = following.ends(|layer| layers[layer].rect, properties, canvas);
    let Geometry::Arrow { curve, heads, .. } = shape.geometry else {
        return None;
    };
    Some(Outline::Arrow(scorsese_compositor::shape::Arrow {
        from,
        to,
        curve,
        heads,
    }))
}

/// How an arrow's layer is laid on the canvas at one instant — what turns a
/// point of its raster into a point of the frame.
struct Laid<'p> {
    properties: &'p Properties,
    anchor: Anchor,
    origin: Origin,
    source: Resolution,
}

impl Laid<'_> {
    /// Where `progress` of the way along `line` is on the canvas, and which
    /// way the line heads there — through the arrow layer's own transform, so
    /// an arrow that is itself moved, turned or mirrored is followed where it
    /// is drawn.
    fn station(&self, line: Outline, progress: f64, canvas: Resolution) -> Option<Station> {
        // NaN reads as the tail, as it does for a trim; `at` clamps the rest.
        let progress = if progress.is_nan() { 0.0 } else { progress };
        let at = measure(&line, canvas)?.at(progress as f32);
        let map = |point| {
            on_canvas(
                self.properties,
                self.anchor,
                self.origin,
                self.source,
                canvas,
                point,
            )
        };
        let position = map(at.position);
        // The tangent is a direction, so it is mapped as the difference of two
        // points a pixel apart: a scale or a mirror on the arrow's layer turns
        // it exactly as it turns the line.
        let ahead = map((at.position.0 + at.tangent.0, at.position.1 + at.tangent.1));
        let (dx, dy) = (ahead.0 - position.0, ahead.1 - position.1);
        let length = dx.hypot(dy);
        let tangent = if length > 0.0 {
            (dx / length, dy / length)
        } else {
            at.tangent
        };
        Some(Station { position, tangent })
    }
}

/// Puts `follower` on `station`: its content's origin point on the line, its
/// own position an offset from there, and — when it orients — the line's
/// heading added to its rotation.
fn ride(follower: &mut Properties, rect: Rect, station: Station, orient: bool, canvas: Resolution) {
    if orient {
        follower.rotation += f64::from(station.heading());
    }
    // Where the point that rides the line lands with no offset at all, under
    // the scale and turn the layer has at this instant. The transform is the
    // compositor's own, so the translation that closes the gap is exact.
    let offset = follower.position;
    follower.position = (0.0, 0.0);
    let (across, down) = rect.origin.fractions();
    let point = rect.area.at(across, down);
    let landed = on_canvas(
        follower,
        rect.anchor,
        rect.origin,
        rect.source,
        canvas,
        point,
    );
    // A position is measured inward from the edges the layer is anchored to,
    // so a far edge counts the other way — the compositor's own rule.
    let inward = |far: bool| if far { -1.0 } else { 1.0 };
    let dx = f64::from(station.position.0 - landed.0) / f64::from(canvas.width());
    let dy = f64::from(station.position.1 - landed.1) / f64::from(canvas.height());
    follower.position = (
        offset.0 + dx * inward(rect.anchor.x == AnchorX::Right),
        offset.1 + dy * inward(rect.anchor.y == AnchorY::Bottom),
    );
}
