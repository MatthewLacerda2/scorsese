//! The native graphics are frozen, and this file is what holds them frozen.
//!
//! > **New motion graphics are pages; the native graphics are frozen** (the
//! > maintainer, 2026-10-06). `shape`, `icon`, arrows with `attach` /
//! > `follow`, text `reveal` / `number` and gradients are frozen: bugs are
//! > fixed, nothing is added. A new graphics capability (a text shadow, a new
//! > gradient, a title preset) goes to pages and their motion kit (#812),
//! > never to the native kinds.
//! >
//! > — `CLAUDE.md`, *Architecture — decided, do not redesign*
//!
//! **If this file stopped compiling, you added to a frozen type.** Every enum
//! below is matched with no wildcard and every struct is destructured with no
//! `..`, so a new variant or field fails right here. A star shape, a conic
//! gradient, a new reveal unit or a third locale is a page (`docs/pages.md`),
//! not a variant. The one legitimate reason to edit this file is a **bug fix**
//! that needs a field: update the pin and say why in the pull request (#1225).
//! When the native kinds are removed (after #797), this file goes with them.
//!
//! Nothing here runs anything worth asserting; the check is the compile.

use scorsese_core::{
    Attach, Counter, Curve, Endpoint, Fill, Follow, Geometry, Heads, Icon, Linear, Locale, Point,
    Radial, Reveal, RevealUnit, Shape, Side, Stop,
};

fn shape(shape: &Shape) {
    let Shape {
        geometry,
        fill,
        stroke: _,
        stroke_width: _,
        dash: _,
    } = shape;
    self::geometry(geometry);
    if let Some(fill) = fill {
        self::fill(fill);
    }
}

fn geometry(geometry: &Geometry) {
    match geometry {
        Geometry::Rectangle {
            width: _,
            height: _,
            radius: _,
        }
        | Geometry::Ellipse {
            width: _,
            height: _,
        } => {}
        Geometry::Arrow {
            from,
            to,
            curve,
            heads,
        } => {
            endpoint(from);
            endpoint(to);
            self::curve(*curve);
            self::heads(*heads);
        }
    }
}

fn endpoint(endpoint: &Endpoint) {
    match endpoint {
        Endpoint::At(at) => point(*at),
        Endpoint::Attached { attach } => {
            let Attach { clip: _, side } = attach;
            self::side(*side);
        }
    }
}

fn point(point: Point) {
    let Point { x: _, y: _ } = point;
}

fn side(side: Side) {
    match side {
        Side::Left | Side::Right | Side::Top | Side::Bottom | Side::Center => {}
    }
}

fn curve(curve: Curve) {
    match curve {
        Curve::Straight | Curve::S => {}
    }
}

fn heads(heads: Heads) {
    match heads {
        Heads::None | Heads::End | Heads::Both => {}
    }
}

fn fill(fill: &Fill) {
    match fill {
        Fill::Solid(_) => {}
        Fill::Linear(Linear { angle: _, stops }) => stops.iter().for_each(stop),
        Fill::Radial(Radial {
            center,
            radius: _,
            stops,
        }) => {
            point(*center);
            stops.iter().for_each(stop);
        }
    }
}

fn stop(stop: &Stop) {
    let Stop { color: _, at: _ } = stop;
}

fn icon(icon: &Icon) {
    let Icon {
        name: _,
        size: _,
        color: _,
        stroke_width: _,
    } = icon;
}

fn follow(follow: &Follow) {
    let Follow { clip: _, orient: _ } = follow;
}

fn reveal(reveal: Reveal) {
    let Reveal {
        unit,
        rise: _,
        stagger: _,
    } = reveal;
    match unit {
        RevealUnit::Char | RevealUnit::Word | RevealUnit::Line => {}
    }
}

fn counter(counter: Counter) {
    let Counter {
        value: _,
        decimals: _,
        locale,
        grouping: _,
    } = counter;
    match locale {
        Locale::En | Locale::PtBr => {}
    }
}

/// Names every pin, so none of them is dead code a lint could ask to delete.
#[test]
fn the_frozen_native_graphics_have_not_grown() {
    let _: fn(&Shape) = shape;
    let _: fn(&Icon) = icon;
    let _: fn(&Follow) = follow;
    let _: fn(Reveal) = reveal;
    let _: fn(Counter) = counter;
}
