//! One block on a lane: what it is, whether anybody has made it, and what a
//! tool has written across it.
//!
//! Three things are said about every clip, in the order somebody reads them:
//!
//! 1. **What kind of thing it is**, as a hue — the same hue the pool uses for
//!    the same asset, and the same the web editor's timeline uses, so the
//!    panels and the two apps are telling one story. A clip that exists is a
//!    solid block of it with white text, as the web draws one.
//! 2. **Whether it exists**, as hatching. A clip whose asset is a brief nobody
//!    has paid for renders as a slug card, and *which* those are is the
//!    question asked before pressing the button that spends money — so an
//!    unmade clip is only tinted with its hue, and hatched. The web does not
//!    draw this yet; it is the one thing here the web has no equivalent of.
//! 3. **What animates it**, as a line drawn through it. A volume ramp is in the
//!    document already and was drawn nowhere — so a duck written by `duck_music`
//!    was invisible until somebody clicked the clip and read a row that said
//!    `volume · 4 points`.

use egui::{Align2, Color32, Painter, Rect, Stroke, StrokeKind, pos2, vec2};
use scorsese_core::{Asset, Clip, Frames};

use super::Paint;
use crate::theme::palette::{self, Palette};
use crate::theme::{ROUND_SM, marks};
use crate::timeline::view::View;

/// How far a clip's text and its ramp stand in from the block's top edge.
const INSET: f32 = 5.0;
/// How far apart the hatching is on a clip nobody has made yet.
///
/// Wide enough to read as a texture rather than as a fill, and pale enough that
/// the clip's own label still sits on top of it — the hatching says *this is
/// indicated*, and a clip you cannot read the name of has said it too loudly.
const HATCH: f32 = 9.0;
/// Below this width there is no room for text, and a clipped glyph reads as a
/// rendering fault rather than as a small clip.
const READABLE: f32 = 30.0;

/// Where a clip sits in its lane.
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

    body(&painter, rect, hue, made, chosen, colours);
    // White on a solid block, as the web sets it; the theme's own text on a
    // tinted one, which is nearly the lane.
    let ink = if made {
        Color32::WHITE
    } else {
        colours.foreground
    };
    ramp(&painter, rect, lane, clip, ink);
    if chosen {
        painter.rect_stroke(
            rect,
            ROUND_SM,
            Stroke::new(2.0, colours.foreground),
            StrokeKind::Inside,
        );
    }
    caption(&painter, (rect, lane), clip, asset, ink, colours);
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

/// The block itself: solid when it exists, tinted and hatched when it is a
/// brief.
///
/// Solid at nine-tenths, and whole when selected, as the web draws a clip
/// (`opacity-90`, and none on the chosen one) — together with the ring drawn
/// over it, that is what picks a selected clip out from a neighbour of the
/// same kind touching it, which is the ordinary case on a cut.
fn body(painter: &Painter, rect: Rect, hue: Color32, made: bool, chosen: bool, colours: &Palette) {
    let lane = colours.muted;
    if made {
        let strength = if chosen { 1.0 } else { 0.9 };
        painter.rect_filled(rect, ROUND_SM, palette::over(hue, lane, strength));
        return;
    }
    painter.rect_filled(rect, ROUND_SM, palette::over(hue, lane, 0.16));
    marks::hatch(painter, rect, palette::over(hue, lane, 0.45), HATCH);
    painter.rect_stroke(
        rect,
        ROUND_SM,
        Stroke::new(1.0, palette::over(hue, lane, 0.7)),
        StrokeKind::Inside,
    );
}

/// The volume ramp, drawn across the clip that carries it.
///
/// Sampled at the pixel rather than drawn from the control points, because the
/// easing between two keyframes is not a straight line and the whole reason to
/// draw this is to see the *shape* of a fade. `KeyframeTrack::value_at` is the
/// same evaluator the mixer uses, so the curve on screen is the curve you hear.
///
/// Volume only, and volume is the honest limit of what a lane this tall can
/// carry: an opacity ramp and a position ramp on one clip would be three lines
/// crossing in thirty pixels. What animates a clip is the inspector's list; what
/// a lane draws is the one property somebody balances by eye.
fn ramp(painter: &Painter, rect: Rect, lane: Rect, clip: &Clip, ink: Color32) {
    let Some(track) = clip
        .keyframes
        .iter()
        .find(|track| track.property.as_str() == "volume")
    else {
        return;
    };
    if rect.width() < READABLE {
        return;
    }
    // The band the curve travels in: under the top edge, and clear of the
    // bottom one so a ramp at zero is still a line rather than the outline.
    let top = rect.top() + INSET;
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
        let frame = Frames((f64::from(along) * clip.duration.get() as f64) as u64);
        track.value_at(frame).unwrap_or(1.0)
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

/// What the clip says on it: the asset it shows, and — when it is a brief —
/// how far along that brief is.
fn caption(
    painter: &Painter,
    (rect, lane): (Rect, Rect),
    clip: &Clip,
    asset: Option<&Asset>,
    ink: Color32,
    colours: &Palette,
) {
    let seen = rect.intersect(lane);
    if seen.width() < READABLE {
        return;
    }
    let baseline = rect.top() + INSET;
    // Pinned to whichever edge is visible rather than to the block's own left,
    // so a long clip scrolled halfway off the side still says what it is —
    // which is exactly when you most want to know.
    marks::label(
        painter,
        pos2(seen.left() + 6.0, baseline),
        Align2::LEFT_TOP,
        clip.asset.as_str(),
        11.0,
        ink,
    );
    // Only for something that does not exist yet. "generated" on a clip that
    // plays is a word taking up room to say that everything is normal.
    if asset.is_some_and(Asset::has_renderable_media) {
        return;
    }
    let Some(state) = asset.and_then(|asset| asset.state) else {
        return;
    };
    marks::label(
        painter,
        pos2(seen.right() - 6.0, baseline),
        Align2::RIGHT_TOP,
        &format!("{state:?}").to_lowercase(),
        10.0,
        colours.muted_foreground,
    );
}
