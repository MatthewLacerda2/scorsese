//! How a `text` asset looks: the style it carries alongside its content.
//!
//! **This is the generality rule in its plainest form.** Core says that text
//! *has* a font, a size, a colour, an alignment; it never says which. There is
//! no "title style" and no "make it red" — there is a font nobody in this crate
//! has an opinion about, and a colour that is whatever the document says. Two
//! faces ship with the compositor so that "choose a font" means something on a
//! fresh install, and a project may name a font file of its own instead.
//!
//! **Sizes here are fractions of the raster, not pixels.** Resolution is a
//! render setting — the same project is previewed at 640×360 and delivered at
//! 4K — so a title written as `72` pixels would be a different title in each.
//! `size` is a fraction of the frame's *height* and `max_width` a fraction of
//! its *width*, which means one number reads the same at every resolution. The
//! compositor works in pixels, because a raster is the only place pixels exist;
//! turning one into the other is `scorsese-render`'s job, where the raster is
//! known.
//!
//! Nothing here animates. A title that grows or fades does it through the
//! properties every layer already has — `transform.scale.*` and `opacity` —
//! rather than through a second, text-shaped way of saying the same thing.

mod font;

pub use font::{DEFAULT_FONT, FontChoice, MAX_WEIGHT, MIN_WEIGHT};

use serde::{Deserialize, Serialize};

use crate::color::Rgba;

/// Which edge of the wrapped block a line of text lines up against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    /// Lines start at the block's left edge — what a paragraph wants.
    Left,
    /// Lines are centred in the block. The default, because the common text
    /// asset is a title or a lower-third rather than a body of prose.
    #[default]
    Center,
    /// Lines end at the block's right edge.
    Right,
}

/// The look of a text asset: font, size, colour, and how it wraps.
///
/// Every field has a default, and an absent `style` means all of them — so a
/// text asset that says nothing about its appearance is a white, centred,
/// sans-serif title, which is the thing most likely to have been meant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct TextStyle {
    /// The face to set the text in: one of the two shipped with scorsese, or a
    /// font file of the project's own.
    pub font: FontChoice,
    /// How heavy to set the glyphs, on the usual scale where 400 is regular
    /// and 700 is bold — read only from a **variable** font file, which is what
    /// most modern open faces now ship as, the two shipped ones included.
    ///
    /// **There is no default for a font the project carries, and that is the
    /// point.** A variable font's own `fvar` table names a default instance,
    /// and it is very often not 400: Manrope's is 200 and Outfit's is 100. A
    /// build that quietly fell back to it would set a title card in hairline
    /// Thin and never say so — a shot rendered wrong that no error mentions,
    /// which is the same failure the `color` asset was deliberately designed
    /// against. So a variable font with no weight named is refused at the point
    /// the file is read, rather than guessed at. A static file has exactly one
    /// weight and needs nothing said about it; naming one there is refused too,
    /// because a field nobody reads is how someone comes to insist their bold
    /// is broken.
    ///
    /// **`sans` and `serif` are the exception, and it is a rule rather than an
    /// exception once the reason is said.** They default to 400. The refusal
    /// above protects against a file scorsese cannot know; these two it ships,
    /// and it publishes their axes. That is the same split the format draws
    /// everywhere — what the document can answer against what only opening the
    /// file can — and it is also what keeps every project ever written valid,
    /// since a weight beside a reserved name used to be refused and so no
    /// existing document carries one.
    ///
    /// Only the `wght` axis is read. Optical size, width and slant are real
    /// axes and none of them is what "make this bold" means.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    /// Whether to set the text in the family's **italic**.
    ///
    /// A boolean and not an angle, because a real italic is a *different
    /// drawing* rather than the upright leaned over — different letterforms,
    /// often a single-storey `a` and an entirely redrawn `f`. A number would
    /// promise a continuum between the two that does not exist.
    ///
    /// It applies to a family scorsese ships, which carries its italic beside
    /// its upright. A font the **project** carries is one file and is whatever
    /// it is, so `italic` beside one is refused rather than ignored — the way
    /// to get an italic there is to name the italic file. Same rule as a weight
    /// beside a static file, and for the same reason: a field nobody reads is
    /// how someone comes to insist their italic is broken.
    pub italic: bool,
    /// The em size, as a fraction of the **frame's height**. `0.1` is a tenth
    /// of the picture — a title — and means the same thing at every render
    /// resolution.
    pub size: f64,
    /// What colour to draw the glyphs, alpha included. Composited like any
    /// other pixels, so a half-transparent caption over a shot works.
    pub color: Rgba,
    /// Which edge the lines line up against, within the wrapped block.
    pub align: TextAlign,
    /// The distance between one baseline and the next, as a multiple of
    /// [`TextStyle::size`]. `1.0` sets lines solid; the default leaves the
    /// small gap that reads as a paragraph.
    pub line_height: f64,
    /// How wide the text may run before it wraps, as a fraction of the
    /// **frame's width**. The default leaves a margin down each side, so a
    /// title never touches the edge of the picture.
    pub max_width: f64,
    /// What colour to rim the glyphs with, alpha included. Absent means no
    /// edge at all, which is what a title over a black plate wants and what
    /// every document written without this field says.
    ///
    /// It is the one thing that makes a caption survive whatever is behind it.
    /// A burned-in line over footage is the main way a video reaches somebody
    /// scrolling with the sound off, and without an edge the only ways to keep
    /// it legible are an opaque plate — the look the caption was avoiding — or
    /// eight offset copies of the same words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Rgba>,
    /// How thick that rim is, as a fraction of the raster's **height** — the
    /// unit [`TextStyle::size`] and a shape's `stroke_width` already use, and
    /// for the same reason: a thickness has no axis of its own, so it needs one
    /// chosen for it, and choosing the same one three times is two fewer things
    /// to remember.
    ///
    /// **The geometry is not a shape's, and that is deliberate.** A shape's
    /// border straddles its outline, half inside and half out; this rim is
    /// added entirely *outside* the letterform, which is then drawn whole on
    /// top of it. Half a width eating inward is exactly the half a caption
    /// cannot spare — it closes the eye of an `e` and the bowl of an `a` at the
    /// sizes captions are actually set at, and the failure is invisible in the
    /// document and shows up only as mush on a finished video. So `text` and
    /// `shape` share the field names and the unit, and nothing else.
    pub stroke_width: f64,
}

impl TextStyle {
    /// A tenth of the frame's height. Big enough to read on a phone, which is
    /// the floor a title has to clear.
    pub const DEFAULT_SIZE: f64 = 0.1;

    /// A twentieth of the frame's width kept clear down each side.
    pub const DEFAULT_MAX_WIDTH: f64 = 0.9;

    /// Loose enough that two lines of a paragraph do not touch.
    pub const DEFAULT_LINE_HEIGHT: f64 = 1.25;

    /// How thick a rim is when the document gives it a colour and no width:
    /// about two pixels at 1080p, and — because this rim grows outward only —
    /// exactly as much ink outside the path as a shape's own default border
    /// puts there. The same edge, measured the way each of them is drawn.
    pub const DEFAULT_STROKE_WIDTH: f64 = 0.002;
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: FontChoice::default(),
            weight: None,
            italic: false,
            size: Self::DEFAULT_SIZE,
            color: Rgba::WHITE,
            align: TextAlign::default(),
            line_height: Self::DEFAULT_LINE_HEIGHT,
            max_width: Self::DEFAULT_MAX_WIDTH,
            stroke: None,
            stroke_width: Self::DEFAULT_STROKE_WIDTH,
        }
    }
}
