//! Clips that travel along an arrow: where on the frame `follow.progress` puts
//! them, one instant at a time.
//!
//! **The second place one layer's geometry depends on another's**, after an
//! arrow attached to a box ([`super::attach`]), and it is paid for the same
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
use scorsese_core::{Anchor, AnchorX, AnchorY, Clip, Frames, Geometry, Origin, Shape};

use crate::content::elapsed;

use super::layers::{Entry, Pixels, Slot};
use super::redraw::Redraw;

/// One clip that travels along an arrow, and where that arrow is.
pub(super) struct Rider {
    /// Which layer of the segment travels.
    pub(super) layer: usize,
    /// Where the line it travels along comes from.
    path: Path,
    /// Whether it turns to face along the line.
    orient: bool,
}

/// Where a follower's line is, once the segment is known.
enum Path {
    /// The arrow is a layer of this segment, at this index.
    Layer(usize),
    /// The arrow is not on screen here and runs between two fixed places, so
    /// its line is read off the document: its clip, for the transform it would
    /// have at this instant, and its shape, for the line itself.
    Absent {
        /// The arrow's clip.
        clip: Clip,
        /// The arrow's shape.
        shape: Shape,
    },
    /// There is no line to follow here — an attached arrow off screen, or on
    /// screen with nothing to point at — so the follower is not drawn.
    Lost,
}

/// Every follower in this segment, and the arrow each one travels along.
///
/// A follower whose arrow is lost comes back with the arrow's clip id, so the
/// caller can say so; `project` is where an arrow that is not on screen is
/// looked for. Validation has already refused an arrow that is missing, is not
/// an arrow, or sits across a group's edge, so a lookup that fails here is a
/// document that did not validate, and its follower is simply left alone.
pub(super) fn riders(
    entries: &[Entry<'_, '_>],
    slots: &[Slot],
    project: &scorsese_core::Project,
) -> (Vec<Rider>, Vec<(String, String)>) {
    let mut riders = Vec::new();
    let mut lost = Vec::new();
    for (layer, entry) in entries.iter().enumerate() {
        let Some(follow) = &entry.shot.clip.follow else {
            continue;
        };
        let on_screen = entries
            .iter()
            .position(|other| other.within == entry.within && other.shot.clip.id == follow.clip);
        let path = match on_screen {
            Some(arrow) => on_screen_path(arrow, entries, slots),
            None => absent_path(project, &follow.clip),
        };
        let Some(path) = path else {
            continue;
        };
        if matches!(path, Path::Lost) {
            lost.push((entry.shot.clip.id.to_string(), follow.clip.to_string()));
        }
        riders.push(Rider {
            layer,
            path,
            orient: follow.orient,
        });
    }
    (riders, lost)
}

/// The line of an arrow drawn in this segment: followed wherever it is drawn,
/// and lost when it is an attached arrow drawing nothing for want of a target.
fn on_screen_path(arrow: usize, entries: &[Entry<'_, '_>], slots: &[Slot]) -> Option<Path> {
    let shape = entries[arrow].shot.asset.shape.as_ref()?;
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return None;
    };
    let attached = from.attach().is_some() || to.attach().is_some();
    let following = matches!(
        &slots[arrow].pixels,
        Pixels::Drawn { redraw, .. } if matches!(**redraw, Redraw::Following(_))
    );
    Some(if attached && !following {
        Path::Lost
    } else {
        Path::Layer(arrow)
    })
}

/// The line of an arrow that is not on screen here, read off the document.
fn absent_path(project: &scorsese_core::Project, id: &scorsese_core::ClipId) -> Option<Path> {
    let (_, clip) = project.every_clip().find(|(_, clip)| &clip.id == id)?;
    let shape = project.asset(&clip.asset)?.shape.as_ref()?;
    let Geometry::Arrow { from, to, .. } = &shape.geometry else {
        return None;
    };
    Some(if from.attach().is_some() || to.attach().is_some() {
        Path::Lost
    } else {
        Path::Absent {
            clip: clip.clone(),
            shape: shape.clone(),
        }
    })
}

/// Moves every follower to its place on its arrow at this instant.
///
/// `properties` is the whole segment's, freshly resolved; `local` is the
/// instant on each layer's own timeline, which is also its arrow's, since the
/// two share one. Runs before any per-frame layer is redrawn, so an arrow
/// attached to a follower meets it where it has got to.
pub(super) fn place(
    riders: &[Rider],
    slots: &[Slot],
    entries: &[Entry<'_, '_>],
    properties: &mut [Properties],
    local: impl Fn(usize) -> Frames,
    canvas: Resolution,
) {
    for rider in riders {
        let progress = properties[rider.layer].progress;
        let station = match &rider.path {
            Path::Layer(arrow) => {
                let line = line_on_screen(*arrow, slots, entries, properties, canvas);
                let slot = &slots[*arrow];
                let lay = Laid {
                    properties: &properties[*arrow],
                    anchor: slot.anchor,
                    origin: slot.origin,
                    source: slot.rect.source,
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
            Some(station) => ride(follower, &slots[rider.layer], station, rider.orient, canvas),
            // No line, so no place: a clip parked in the middle of the frame
            // would be a worse answer than one that is plainly absent.
            None => follower.opacity = 0.0,
        }
    }
}

/// The arrow of layer `arrow` as it runs at this instant, in its own raster.
fn line_on_screen(
    arrow: usize,
    slots: &[Slot],
    entries: &[Entry<'_, '_>],
    properties: &[Properties],
    canvas: Resolution,
) -> Option<Outline> {
    let shape = entries[arrow].shot.asset.shape.as_ref()?;
    if let Pixels::Drawn { redraw, .. } = &slots[arrow].pixels
        && let Redraw::Following(following) = &**redraw
    {
        let [from, to] = following.ends(slots, properties, canvas);
        let Geometry::Arrow { curve, heads, .. } = shape.geometry else {
            return None;
        };
        return Some(Outline::Arrow(scorsese_compositor::shape::Arrow {
            from,
            to,
            curve,
            heads,
        }));
    }
    Some(crate::shape::outline(
        &shape.geometry,
        canvas,
        slots[arrow].anchor,
    ))
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
fn ride(
    follower: &mut Properties,
    slot: &Slot,
    station: Station,
    orient: bool,
    canvas: Resolution,
) {
    if orient {
        follower.rotation += f64::from(station.heading());
    }
    // Where the point that rides the line lands with no offset at all, under
    // the scale and turn the layer has at this instant. The transform is the
    // compositor's own, so the translation that closes the gap is exact.
    let offset = follower.position;
    follower.position = (0.0, 0.0);
    let (across, down) = slot.origin.fractions();
    let point = slot.rect.area.at(across, down);
    let landed = on_canvas(
        follower,
        slot.anchor,
        slot.origin,
        slot.rect.source,
        canvas,
        point,
    );
    // A position is measured inward from the edges the layer is anchored to,
    // so a far edge counts the other way — the compositor's own rule.
    let inward = |far: bool| if far { -1.0 } else { 1.0 };
    let dx = f64::from(station.position.0 - landed.0) / f64::from(canvas.width());
    let dy = f64::from(station.position.1 - landed.1) / f64::from(canvas.height());
    follower.position = (
        offset.0 + dx * inward(slot.anchor.x == AnchorX::Right),
        offset.1 + dy * inward(slot.anchor.y == AnchorY::Bottom),
    );
}
