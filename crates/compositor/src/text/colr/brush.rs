//! The colours a colour glyph is made of, and the shaders that lay them down.
//!
//! Two tables and one translation. `CPAL` is the face's own palette — a flat
//! list of colours that every fill in `COLR` refers to by number — and a
//! [`Brush`] is one of those fills, either a single colour or a gradient
//! between several. tiny-skia calls the same things shaders, so all this does
//! is say one in the other's words.
//!
//! **Where it cannot, it says so rather than guessing.** Every gradient the
//! format has is one tiny-skia also has — linear, radial, and the sweep that
//! `SweepGradient` describes — so the translation is total in practice, and the
//! cases below are what a *malformed* file produces: a spread mode nobody
//! defined, a colour outside the palette, geometry no gradient can be built
//! from. Filling those with something plausible would put a wrong colour on a
//! frame with nothing anywhere saying so, which is the failure this whole
//! module exists downstream of.

use skrifa::color::{Brush, ColorStop, Extend};
use skrifa::raw::{FontRef, TableProvider};
use tiny_skia::{
    Color, GradientStop, LinearGradient, Point, RadialGradient, Shader, SpreadMode, SweepGradient,
    Transform,
};

use scorsese_core::Rgba;

/// Something a colour glyph asked for that could not be drawn as asked.
///
/// A note rather than a refusal, and the same shape as an unknown icon name:
/// the rest of the title is still worth having, and one glyph drawn short is
/// worth saying out loud. A render carries these into its report.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Unpaintable {
    /// What the font asked for, in the words somebody reading a report needs.
    pub wanted: String,
}

/// The face's own colours: palette 0 of its `CPAL` table, entry by entry.
///
/// Palette 0 and no other. `CPAL` may carry several — a light one and a dark
/// one, typically — and choosing between them is a property *value* nothing in
/// this editor has asked for. The first is the one the designer means by
/// default, and picking it silently is the same choice every renderer makes.
///
/// Empty for a face with no `CPAL`, which is all eight nameable ones.
pub(in crate::text) fn palette(font: &FontRef<'_>) -> Vec<Rgba> {
    let Ok(cpal) = font.cpal() else {
        return Vec::new();
    };
    let Some(Ok(records)) = cpal.color_records_array() else {
        return Vec::new();
    };
    let first = cpal
        .color_record_indices()
        .first()
        .map_or(0, |index| usize::from(index.get()).min(records.len()));
    let entries = usize::from(cpal.num_palette_entries()).min(records.len() - first);
    records[first..first + entries]
        .iter()
        .map(|record| Rgba {
            r: record.red,
            g: record.green,
            b: record.blue,
            a: record.alpha,
        })
        .collect()
}

/// The colours and gradients of one glyph, resolved against one palette.
pub(super) struct Palette<'a> {
    /// The face's entries, by index.
    pub entries: &'a [Rgba],
    /// What `0xFFFF` means: the colour the text itself is being set in. A
    /// colour glyph that names it is asking to be tinted like a letter, which
    /// is how a font draws a symbol meant to take the caption's colour.
    pub foreground: Rgba,
}

/// The palette index the format reserves for "whatever colour the text is".
const FOREGROUND: u16 = 0xffff;

impl Palette<'_> {
    /// One palette entry at `alpha`, or `None` — with a reason — when the
    /// index is not one the face has.
    fn colour(&self, index: u16, alpha: f32, said: &mut Vec<Unpaintable>) -> Option<Color> {
        let found = if index == FOREGROUND {
            Some(self.foreground)
        } else {
            self.entries.get(usize::from(index)).copied()
        };
        let Some(colour) = found else {
            report(
                said,
                format!("colour {index}, which its own palette has no entry for"),
            );
            return None;
        };
        Some(Color::from_rgba8(
            colour.r,
            colour.g,
            colour.b,
            // The format's alpha multiplies the palette entry's own rather
            // than replacing it, so a half-transparent entry stays half.
            (f32::from(colour.a) * alpha.clamp(0.0, 1.0)) as u8,
        ))
    }

    fn stops(&self, stops: &[ColorStop], said: &mut Vec<Unpaintable>) -> Vec<GradientStop> {
        stops
            .iter()
            .filter_map(|stop| {
                let colour = self.colour(stop.palette_index, stop.alpha, said)?;
                Some(GradientStop::new(stop.offset, colour))
            })
            .collect()
    }
}

/// The shader that draws `brush`, in the space `transform` maps to the raster.
///
/// `None` is a brush that could not be built at all, and every path to it has
/// already said why.
pub(super) fn shader(
    brush: &Brush<'_>,
    palette: &Palette<'_>,
    transform: Transform,
    said: &mut Vec<Unpaintable>,
) -> Option<Shader<'static>> {
    match brush {
        Brush::Solid {
            palette_index,
            alpha,
        } => Some(Shader::SolidColor(palette.colour(
            *palette_index,
            *alpha,
            said,
        )?)),
        Brush::LinearGradient {
            p0,
            p1,
            color_stops,
            extend,
        } => built(
            LinearGradient::new(
                at(*p0),
                at(*p1),
                palette.stops(color_stops, said),
                spread(*extend, said),
                transform,
            ),
            said,
        ),
        Brush::RadialGradient {
            c0,
            r0,
            c1,
            r1,
            color_stops,
            extend,
        } => built(
            RadialGradient::new(
                at(*c0),
                // A normalised colour line can put a radius below zero, which
                // no circle has. Clamped rather than refused: the stops either
                // side of it are still the gradient the designer drew.
                r0.max(0.0),
                at(*c1),
                r1.max(0.0),
                palette.stops(color_stops, said),
                spread(*extend, said),
                transform,
            ),
            said,
        ),
        Brush::SweepGradient {
            c0,
            start_angle,
            end_angle,
            color_stops,
            extend,
        } => built(
            SweepGradient::new(
                at(*c0),
                *start_angle,
                *end_angle,
                palette.stops(color_stops, said),
                spread(*extend, said),
                transform,
            ),
            said,
        ),
    }
}

/// A gradient tiny-skia refused to build — degenerate geometry, or no stops
/// left after a palette that could not answer.
fn built(shader: Option<Shader<'static>>, said: &mut Vec<Unpaintable>) -> Option<Shader<'static>> {
    if shader.is_none() {
        report(
            said,
            "a gradient with no shape tiny-skia can build".to_owned(),
        );
    }
    shader
}

fn at(point: skrifa::raw::types::Point<f32>) -> Point {
    Point::from_xy(point.x, point.y)
}

/// How a gradient carries on past its last stop.
fn spread(extend: Extend, said: &mut Vec<Unpaintable>) -> SpreadMode {
    match extend {
        Extend::Repeat => SpreadMode::Repeat,
        Extend::Reflect => SpreadMode::Reflect,
        Extend::Pad => SpreadMode::Pad,
        // Only a malformed file reaches this: the format defines three.
        _ => {
            report(
                said,
                "a gradient spread this build does not know".to_owned(),
            );
            SpreadMode::Pad
        }
    }
}

/// Records something that could not be drawn, once however often it happens.
///
/// Once, because the finding is about the font and the instruction, not about
/// how many glyphs on the frame ran into it — a report naming the same
/// sentence forty times is a report nobody finishes.
pub(super) fn report(said: &mut Vec<Unpaintable>, wanted: String) {
    let note = Unpaintable { wanted };
    if !said.contains(&note) {
        said.push(note);
    }
}
