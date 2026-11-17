//! Drawing an interface in the units its layouts are written in.
//!
//! Every layout is written against a 1080-pixel-wide phone screen: a
//! measurement is in those pixels, scaled to the frame's width, and a control
//! near the foot of the screen is placed from the bottom edge, so the same
//! layout sits right on a 1080×1920 render and on a contact sheet's cell.

use std::sync::OnceLock;

use scorsese_compositor::text::{Font, Slant, draw_line};
use scorsese_compositor::{Area, Frame, icon, interface};
use scorsese_core::Rgba;

/// The width every layout is written against.
const DESIGN_WIDTH: f32 = 1080.0;

/// The colour of the interface's own marks.
pub(super) const WHITE: Rgba = rgba(255, 255, 255, 255);
/// The second line of a control: a count, a caption.
pub(super) const SOFT: Rgba = rgba(255, 255, 255, 200);
/// The dark of a navigation bar and a filled button's label.
pub(super) const INK: Rgba = rgba(18, 18, 18, 255);
/// A placeholder avatar.
pub(super) const AVATAR: Rgba = rgba(150, 150, 150, 255);
/// A translucent button's fill.
pub(super) const GLASS: Rgba = rgba(255, 255, 255, 64);

/// A frame, and the scale of its layout's units to its pixels.
pub(super) struct Pen<'f> {
    frame: &'f mut Frame,
    scale: f32,
    /// The frame's height, in layout units.
    pub(super) height: f32,
}

impl<'f> Pen<'f> {
    /// A pen over `frame`, scaled to its width.
    pub(super) fn new(frame: &'f mut Frame) -> Self {
        let resolution = frame.resolution();
        let scale = resolution.width() as f32 / DESIGN_WIDTH;
        Self {
            height: resolution.height() as f32 / scale,
            frame,
            scale,
        }
    }

    /// A height measured up from the bottom edge, as a height from the top.
    pub(super) fn up(&self, from_bottom: f32) -> f32 {
        self.height - from_bottom
    }

    fn area(&self, (x, y, width, height): (f32, f32, f32, f32)) -> Area {
        let s = self.scale;
        Area {
            left: x * s,
            top: y * s,
            width: width * s,
            height: height * s,
        }
    }

    /// The shade from `from` to `to` over the rows from `top`, `height` down.
    pub(super) fn fade(&mut self, top: f32, height: f32, colors: (Rgba, Rgba)) {
        let s = self.scale;
        interface::fade(self.frame, top * s, height * s, colors);
    }

    /// A filled box `(x, y, width, height)`, its corners rounded to `radius`.
    pub(super) fn boxed(&mut self, rect: (f32, f32, f32, f32), radius: f32, color: Rgba) {
        let area = self.area(rect);
        interface::rounded(self.frame, area, radius * self.scale, color);
    }

    /// The outline of a box, `width` thick.
    pub(super) fn outline(
        &mut self,
        rect: (f32, f32, f32, f32),
        radius: f32,
        (color, width): (Rgba, f32),
    ) {
        let (area, s) = (self.area(rect), self.scale);
        interface::outlined(self.frame, area, radius * s, (color, width * s));
    }

    /// A filled circle.
    pub(super) fn disc(&mut self, (x, y): (f32, f32), radius: f32, color: Rgba) {
        let s = self.scale;
        interface::disc(self.frame, (x * s, y * s), radius * s, color);
    }

    /// A circle's outline, `width` thick.
    pub(super) fn ring(&mut self, (x, y): (f32, f32), radius: f32, (color, width): (Rgba, f32)) {
        let s = self.scale;
        interface::ring(self.frame, (x * s, y * s), radius * s, (color, width * s));
    }

    /// The symbol `name` in a square of `size` centred on `(x, y)`, in Lucide's
    /// own weight. A name the set lacks draws nothing, and fails every test
    /// that draws it.
    pub(super) fn icon(&mut self, name: &str, (x, y): (f32, f32), size: f32, color: Rgba) {
        let found = icon::find(name);
        #[cfg(test)]
        assert!(found.is_some() || name.is_empty(), "no symbol {name}");
        let Some(found) = found else {
            return;
        };
        let s = self.scale;
        let corner = ((x - size / 2.0) * s, (y - size / 2.0) * s);
        let width = size * s * 2.0 / icon::VIEWBOX;
        interface::symbol(self.frame, found, corner, size * s, (color, width));
    }

    /// One line of type starting at `x`, sitting on the baseline `y`. Hands
    /// back where it ends.
    pub(super) fn text(
        &mut self,
        text: &str,
        (x, y): (f32, f32),
        size: f32,
        (bold, color): (bool, Rgba),
    ) -> f32 {
        let s = self.scale;
        let font = face(bold);
        draw_line(self.frame, text, font, size * s, color, (x * s, y * s));
        x + interface::line_width(font, text, size * s) / s
    }

    /// One line of type centred on `x`.
    pub(super) fn centred(
        &mut self,
        text: &str,
        (x, y): (f32, f32),
        size: f32,
        style: (bool, Rgba),
    ) {
        let width = self.width_of(text, size, style.0);
        self.text(text, (x - width / 2.0, y), size, style);
    }

    /// How wide `text` sets, in layout units.
    pub(super) fn width_of(&self, text: &str, size: f32, bold: bool) -> f32 {
        let s = self.scale;
        interface::line_width(face(bold), text, size * s) / s
    }
}

/// The shipped sans, bold for a name or a button, regular otherwise.
fn face(bold: bool) -> &'static Font {
    static BOLD: OnceLock<Option<Font>> = OnceLock::new();
    if !bold {
        return Font::sans();
    }
    BOLD.get_or_init(|| Font::shipped("sans", Some(700), Slant::Upright).ok())
        .as_ref()
        .unwrap_or_else(Font::sans)
}

/// A colour, written as a constant.
pub(super) const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba {
    Rgba { r, g, b, a }
}
