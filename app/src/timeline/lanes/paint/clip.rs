//! One block on a lane: what it is, whether anybody has made it, and what a
//! tool has written across it.
//!
//! `web/src/editor/timeline/Lane.tsx`, `ClipFill.tsx` and `Unmade.tsx`, in
//! egui. Every clip fills its lane top to bottom and wears a 1px border in its
//! kind's hue; what is inside says whether it exists:
//!
//! 1. **A made clip** is filled by its kind: the asset's picture tiled along
//!    it, a sound's waveform, or the solid hue.
//! 2. **An unmade clip** — a brief nobody has paid for, which renders as a slug
//!    card — is tinted and hatched with its hue, and says how far along it is.
//!    *Which* clips those are is the question asked before pressing the button
//!    that spends money.
//! 3. **What animates it** is a line drawn through it. A volume ramp is in the
//!    document already; a duck written by `duck_music` would otherwise be
//!    invisible until somebody clicked the clip and read a row.
//!
//! Both wear the asset's name on a dark strip top-left and a trim strip at each
//! end, and a selected clip a ring outside it.

use std::sync::Arc;

use egui::{
    Align2, Color32, CornerRadius, Painter, Rect, Stroke, StrokeKind, TextureHandle, pos2, vec2,
};
use scorsese_core::{Asset, Clip, Project};
use scorsese_render::audio::{Gain, Peaks};

use super::Paint;
use crate::theme::palette::{self, Palette};
use crate::theme::{ROUND_SM, marks};
use crate::timeline::fill::Fill;
use crate::timeline::sound;
use crate::timeline::view::View;

/// How tall the name strip is: the web's `leading-4`.
const STRIP: f32 = 16.0;
/// How far apart the hatching is on a clip nobody has made yet.
const HATCH: f32 = 9.0;
/// How wide the trim strip at each end of a clip is.
const TRIM: f32 = 6.0;
/// Below this width there is no room for text, and a clipped glyph reads as a
/// rendering fault rather than as a small clip.
const READABLE: f32 = 30.0;
/// How much room the state word of an unmade clip keeps clear on the name
/// strip's right: the web's `pr-10`.
const STATE_ROOM: f32 = 40.0;

/// Where a clip sits in its lane: the whole lane's height, as the web's
/// `inset-y-0`.
pub(in crate::timeline::lanes) fn rect(lane: Rect, clip: &Clip, view: View) -> Rect {
    let left = lane.left() + view.offset_of(clip.start);
    // At least a hair wide, so a very short clip at a wide zoom is still
    // something you can see and click rather than nothing at all.
    let width = view.width_of(clip.duration).max(2.0);
    Rect::from_min_size(pos2(left, lane.top()), vec2(width, lane.height()))
}

/// Draws one clip whole.
///
/// `rect` is the block's **whole** rectangle, including whatever of it is
/// scrolled off the side, and `lane` is what may actually be painted. Clipped
/// rather than shrunk, and the difference is not cosmetic: a clip cut down to
/// the visible strip draws a rounded corner at the edge of the view, which says
/// *the clip ends here* about a clip that does not — and it would put a volume
/// ramp's whole shape into the sliver that happens to be on screen.
pub(in crate::timeline::lanes) fn draw(paint: &Paint<'_>, rect: Rect, lane: Rect, clip: &Clip) {
    let painter = paint.painter.with_clip_rect(lane);
    let colours = palette::of(painter.ctx());
    let asset = paint.project.asset(&clip.asset);
    let hue = tint(paint, asset, colours);
    let made = asset.is_some_and(Asset::has_renderable_media);
    let chosen = paint.selected.contains(&clip.id);

    if made {
        let fill = asset.and_then(|asset| paint.fills.get(&asset.id));
        body(&painter, rect, hue, chosen, colours);
        fill_in(&painter, rect, fill, (clip, paint.project), chosen);
    } else {
        unmade(&painter, rect, hue, colours);
    }
    painter.rect_stroke(rect, ROUND_SM, Stroke::new(1.0, hue), StrokeKind::Inside);
    // The name strip sits over the picture, and the trim strips over that.
    caption(&painter, (rect, lane), clip, asset, made, colours);
    trims(&painter, rect);
    // White on a solid block, as the web sets it; the theme's own text on a
    // tinted one, which is nearly the lane.
    let ink = if made {
        palette::ON_HUE
    } else {
        colours.foreground
    };
    ramp(&painter, rect, lane, clip, ink);
    if chosen {
        // Outside the clip, so it may stand over a neighbouring lane's edge.
        paint
            .painter
            .with_clip_rect(lane.expand2(vec2(0.0, 2.0)))
            .rect_stroke(
                rect,
                ROUND_SM,
                Stroke::new(2.0, colours.foreground),
                StrokeKind::Outside,
            );
    }
}

/// The hue this clip is drawn in, dimmed when the pool is pointing at some
/// other asset.
///
/// Dimming everything else rather than outlining the few keeps the answer to
/// "where is this used?" readable when an asset is used twenty times.
fn tint(paint: &Paint<'_>, asset: Option<&Asset>, colours: &Palette) -> Color32 {
    let hue = asset.map_or(palette::UNKNOWN, |asset| palette::of_kind(asset.kind));
    match &paint.highlighted {
        Some(picked) if Some(picked) != asset.map(|asset| &asset.id) => {
            palette::over(hue, colours.muted, 0.32)
        }
        _ => hue,
    }
}

/// A made clip's ground: the solid hue at nine-tenths, and whole when
/// selected, as the web draws a clip (`opacity-90`, and none on the chosen
/// one) — together with the ring drawn over it, that is what picks a selected
/// clip out from a neighbour of the same kind touching it, which is the
/// ordinary case on a cut.
fn body(painter: &Painter, rect: Rect, hue: Color32, chosen: bool, colours: &Palette) {
    let strength = if chosen { 1.0 } else { 0.9 };
    painter.rect_filled(rect, ROUND_SM, palette::over(hue, colours.muted, strength));
}

/// An unmade clip: `muted` ground, the hue at 15%, hatching in the hue at 45%
/// and an inset ring (`Unmade.tsx`).
fn unmade(painter: &Painter, rect: Rect, hue: Color32, colours: &Palette) {
    let tinted = palette::over(hue, colours.muted, 0.15);
    painter.rect_filled(rect, ROUND_SM, tinted);
    marks::hatch(painter, rect, palette::over(hue, tinted, 0.45), HATCH, 2.0);
    painter.rect_stroke(
        rect,
        ROUND_SM,
        Stroke::new(1.0, colours.foreground.gamma_multiply(0.2)),
        StrokeKind::Inside,
    );
}

/// What a made clip is filled with beyond its hue: its picture tiled along it
/// at the clip's height, or its sound's waveform (`ClipFill.tsx`).
fn fill_in(
    painter: &Painter,
    rect: Rect,
    fill: Option<&Fill>,
    (clip, project): (&Clip, &Project),
    chosen: bool,
) {
    let inner = rect.shrink(1.0);
    let strength = if chosen { 1.0 } else { 0.9 };
    match fill {
        Some(Fill::Picture(texture)) => tiles(painter, inner, texture, strength),
        Some(Fill::Sound(found)) => waveform(painter, inner, found, (clip, project)),
        None => {}
    }
}

/// The picture repeated along `inner`, each copy as tall as the clip.
fn tiles(painter: &Painter, inner: Rect, texture: &TextureHandle, strength: f32) {
    let size = texture.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 || inner.height() < 1.0 {
        return;
    }
    let wide = (inner.height() * size.x / size.y).max(1.0);
    // Only the tiles anybody can see: a clip is drawn at its whole width even
    // when most of it is scrolled off the side.
    let seen = inner.intersect(painter.clip_rect());
    if seen.width() < 1.0 {
        return;
    }
    let tint = palette::UNTINTED.gamma_multiply(strength);
    let first = ((seen.left() - inner.left()) / wide).floor().max(0.0);
    let mut x = inner.left() + first * wide;
    while x < seen.right() {
        let whole = Rect::from_min_size(pos2(x, inner.top()), vec2(wide, inner.height()));
        let shown = whole.intersect(inner);
        let uv = Rect::from_min_max(
            pos2((shown.left() - x) / wide, 0.0),
            pos2((shown.right() - x) / wide, 1.0),
        );
        painter.image(texture.id(), shown, uv, tint);
        x += wide;
    }
}

/// The sound's levels, a bar every few pixels below the name strip.
fn waveform(
    painter: &Painter,
    inner: Rect,
    found: &Arc<Peaks>,
    (clip, project): (&Clip, &Project),
) {
    let fps = project.timeline_fps;
    let from = fps.seconds(clip.source_in);
    let length = fps.seconds(clip.duration) * clip.speed.get();
    let band = Rect::from_min_max(
        pos2(inner.left(), inner.top() + STRIP - 1.0),
        pos2(inner.right(), inner.bottom() - 1.0),
    );
    if band.height() < 2.0 {
        return;
    }
    let bars = sound::bars_of(found.per_second, length, band.width());
    let levels = sound::slice_of(found, from, length, bars);
    let each = band.width() / bars as f32;
    let seen = band.intersect(painter.clip_rect());
    let first = (((seen.left() - band.left()) / each).floor().max(0.0)) as usize;
    let ink = palette::ON_HUE.gamma_multiply(0.55);
    for (at, level) in levels.iter().enumerate().skip(first) {
        let x = band.left() + at as f32 * each;
        if x > seen.right() {
            break;
        }
        let tall = level.max(0.04) * band.height();
        let bar = Rect::from_center_size(
            pos2(x + each / 3.0, band.center().y),
            vec2(each * 2.0 / 3.0, tall),
        );
        painter.rect_filled(bar, 0.0, ink);
    }
}

/// The trim strips: 6px of `screen/20` at both ends.
fn trims(painter: &Painter, rect: Rect) {
    let strip = palette::TRIM_STRIP;
    let inner = rect.shrink(1.0);
    if inner.width() < TRIM * 2.0 {
        return;
    }
    for x in [inner.left(), inner.right() - TRIM] {
        painter.rect_filled(
            Rect::from_min_size(pos2(x, inner.top()), vec2(TRIM, inner.height())),
            0.0,
            strip,
        );
    }
}

/// The volume ramp, drawn across the clip that carries it.
///
/// Sampled at the pixel rather than drawn from the control points, because the
/// easing between two keyframes is not a straight line and the whole reason to
/// draw this is to see the *shape* of a fade. What is sampled is the
/// mixer's own [`Gain`], so the curve on screen is the curve you hear — every
/// volume track on the clip multiplied, a hand-written fade and a duck alike.
///
/// Volume only, and volume is the honest limit of what a lane this tall can
/// carry: an opacity ramp and a position ramp on one clip would be three lines
/// crossing in thirty pixels. What animates a clip is the inspector's list; what
/// a lane draws is the one property somebody balances by eye.
fn ramp(painter: &Painter, rect: Rect, lane: Rect, clip: &Clip, ink: Color32) {
    let gain = Gain::of(&clip.keyframes);
    if gain.is_unity() {
        return;
    }
    if rect.width() < READABLE {
        return;
    }
    // The band the curve travels in: under the top edge, and clear of the
    // bottom one so a ramp at zero is still a line rather than the outline.
    let top = rect.top() + STRIP;
    let floor = rect.bottom() - 3.0;
    if floor <= top {
        return;
    }

    let height = |value: f64| floor - (value.clamp(0.0, 1.0) as f32) * (floor - top);
    let at = |x: f32| {
        // Where this pixel falls inside the clip. Keyframe times are relative to
        // the clip's start, and `rect` is the block's *whole* rectangle — so a
        // clip half scrolled off the left edge still puts the ramp's corners
        // where they belong rather than squeezing the whole shape into whatever
        // sliver is on screen.
        let along = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        let frame = (f64::from(along) * clip.duration.get() as f64).floor();
        f64::from(gain.at(frame))
    };

    // Only the part of it anybody can see. Sampling the whole block would be
    // tens of thousands of points on a long clip at a deep zoom — a polyline
    // rebuilt on every repaint, which is the repaint the scrub loop cannot
    // afford. The mapping still uses the block's whole rectangle, so what is
    // drawn is a window onto the same curve rather than a different one.
    let (from, to) = (rect.left().max(lane.left()), rect.right().min(lane.right()));
    let line = ink.gamma_multiply(0.7);
    let mut points = Vec::new();
    let mut x = from;
    while x < to {
        points.push(pos2(x, height(at(x))));
        x += 2.0;
    }
    points.push(pos2(to, height(at(to))));
    painter.add(egui::Shape::line(points, Stroke::new(1.0, line)));
}

/// What the clip says on it: the asset's text if it has one, else its id, on a
/// dark strip top-left — and, when it is a brief, how far along that brief is.
fn caption(
    painter: &Painter,
    (rect, lane): (Rect, Rect),
    clip: &Clip,
    asset: Option<&Asset>,
    made: bool,
    colours: &Palette,
) {
    let seen = rect.intersect(lane);
    if seen.width() < READABLE {
        return;
    }
    // Only for something that does not exist yet. "generated" on a clip that
    // plays is a word taking up room to say that everything is normal.
    let state = (!made)
        .then(|| asset.and_then(|asset| asset.state))
        .flatten()
        .map(|state| format!("{state:?}").to_lowercase());
    let name = asset
        .and_then(|asset| asset.text.as_deref())
        .unwrap_or(clip.asset.as_str());
    // Pinned to whichever edge is visible rather than to the block's own left,
    // so a long clip scrolled halfway off the side still says what it is —
    // which is exactly when you most want to know.
    let at = pos2(seen.left() + 1.0, rect.top() + 1.0);
    let room = seen.width() - 2.0 - if state.is_some() { STATE_ROOM } else { 0.0 };
    let galley = marks::truncated(
        painter,
        name,
        marks::caption_font(),
        palette::ON_HUE,
        (room - 8.0).max(0.0),
    );
    let strip = Rect::from_min_size(at, vec2(galley.size().x + 8.0, STRIP));
    painter.rect_filled(
        strip,
        CornerRadius {
            se: ROUND_SM.se,
            ..CornerRadius::ZERO
        },
        palette::NAME_STRIP,
    );
    painter.galley(
        pos2(strip.left() + 4.0, strip.center().y - galley.size().y / 2.0),
        galley,
        palette::ON_HUE,
    );
    if let Some(state) = state {
        marks::label(
            painter,
            pos2(seen.right() - 8.0, rect.top() + 2.0),
            Align2::RIGHT_TOP,
            &state,
            10.0,
            colours.muted_foreground,
        );
    }
}
