//! Turning characters into positioned glyphs.
//!
//! The stage between "these are the letters" and "these are the outlines".
//! It exists for **kerning** — the per-pair adjustment that closes the gap in
//! `AV`, `To`, `Wa` — which is not a table a pair can be looked up in: a font
//! states it as a positioning *feature*, applied to a run by a shaper. Placing
//! each glyph at the previous one's advance, which is what this module
//! replaced, is correct and readable and loose in exactly the pairs a designer
//! notices first, which at title size is most of what this editor sets.
//!
//! **The shaper is HarfRust**, the HarfBuzz algorithm in Rust, reading the
//! same `read-fonts` face skrifa reads. What comes back is a run of glyph ids
//! with positions in font units, which this module scales to the pixels the
//! raster is measured in.
//!
//! What arrives with kerning, because a shaper applies what the font asks for:
//! standard ligatures, contextual alternates, and mark positioning. That is
//! not scope creep, it is the same sentence — a shaped run is what the face's
//! designer specified, and refusing half of it would mean naming features to
//! switch off and defending the list.
//!
//! What does **not** arrive is the layout above it: no bidirectional
//! reordering across a paragraph, no locale-aware line breaking. A run is
//! shaped as one segment, in the direction and script guessed from its own
//! characters, and where the lines break is still [`super::layout`]'s
//! decision.
//!
//! **A line is not necessarily one run.** Which face sets which stretch is
//! [`super::runs`]'s decision, and each stretch is shaped here against that
//! face on its own — so a glyph carries the face it came from, and the shaped
//! runs are laid end to end by [`Shaped::append`].

use harfrust::{Feature, ShapeOptions, Shaper, Tag, UnicodeBuffer};
use skrifa::GlyphId;

/// The non-breaking space.
///
/// Named here because both halves of honouring it live one call apart:
/// [`super::layout`] must not break a line at one, and this module must not
/// drop one for want of a glyph. A face that has no `U+00A0` of its own still
/// has a space, and that is the width the author asked for.
pub(super) const NBSP: char = '\u{a0}';

/// The space as wide as a figure, and the one as wide as a full stop.
///
/// What a counting number is padded with so that the line keeps the width of
/// the widest figure it will reach: a figure space stands where a digit will
/// be, a punctuation space where a separator will. Unicode calls both
/// whitespace, and like [`NBSP`] neither may be broken at or collapsed — they
/// are the room a figure keeps, not a gap between words.
pub(super) const FIGURE_SPACE: char = '\u{2007}';
/// See [`FIGURE_SPACE`].
pub(super) const PUNCTUATION_SPACE: char = '\u{2008}';

/// Whether `character` is a space that holds its width rather than being a
/// place to break: [`NBSP`] and the two a counter pads with.
pub(super) fn holds(character: char) -> bool {
    matches!(character, NBSP | FIGURE_SPACE | PUNCTUATION_SPACE)
}

/// Whether figures are set from the face's tabular forms — every digit the
/// same width — rather than its proportional ones.
///
/// On for a text with a counting number in it, and for the **whole** of that
/// text rather than the figure alone: a run is shaped with one set of features,
/// and a caption whose other digits changed style beside the counter would look
/// like a font change. A face with no `tnum` feature ignores the request and
/// keeps its own figures, and the figure space [`super::padded`] pads with is
/// then measured against those.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Figures {
    /// Whatever the face sets by default — proportional in most text faces.
    #[default]
    Proportional,
    /// The face's `tnum` forms, where it has them.
    Tabular,
}

impl Figures {
    fn features(self) -> &'static [Feature] {
        static TABULAR: [Feature; 1] = [Feature {
            tag: Tag::new(b"tnum"),
            value: 1,
            start: 0,
            end: u32::MAX,
        }];
        match self {
            Self::Proportional => &[],
            Self::Tabular => &TABULAR,
        }
    }
}

/// A run of text, shaped: which glyphs, and where each one goes.
#[derive(Default)]
pub(super) struct Shaped {
    /// The glyphs in the order they are drawn, which for a right-to-left run
    /// is already the visual order the shaper put them in.
    pub glyphs: Vec<Placed>,
    /// How wide the run sets, in pixels — every advance in it, kerning
    /// included. The one measurement the rest of the module uses, so what is
    /// wrapped and what is drawn can never disagree.
    pub width: f32,
}

/// One glyph and where its origin sits, in pixels **relative to the start of
/// the run** — `x` rightwards, `y` upwards as font space has it, which is the
/// convention the outline pen is already flipping.
pub(super) struct Placed {
    /// Which glyph in the face, already chosen: a shaper may have merged two
    /// characters into one glyph or swapped one for a contextual form, so
    /// there is no character to map any more.
    pub id: GlyphId,
    /// Where to put it, relative to the run's origin.
    pub at: (f32, f32),
    /// Which face in the chain drew it — a glyph id means nothing without the
    /// face it indexes into, and a line may be set from more than one.
    pub face: usize,
    /// Where in the shaped string the characters it sets begin, as a byte
    /// offset — which is how a reveal finds the word a glyph belongs to. A
    /// ligature carries the offset of its first character.
    pub cluster: usize,
}

impl Shaped {
    /// Lays `other` immediately after this run, shifting its glyphs by the
    /// width already accumulated.
    ///
    /// Positions rather than a second pen: the runs of one line are one line,
    /// and something asking how wide the whole of it sets must not have to walk
    /// a list of pieces to find out. `from` is where `other`'s text starts in
    /// the line, so its clusters are counted from the line's start too.
    pub(super) fn append(&mut self, other: Self, from: usize) {
        let offset = self.width;
        self.glyphs
            .extend(other.glyphs.into_iter().map(|glyph| Placed {
                at: (glyph.at.0 + offset, glyph.at.1),
                cluster: glyph.cluster + from,
                ..glyph
            }));
        self.width += other.width;
    }
}

/// Shapes `text` with `shaper`, scaling font units to pixels by `scale`.
///
/// Positions are accumulated here rather than read off the shaper's own pen,
/// so that a glyph the face cannot draw can be dropped **with its advance**.
/// A character the face has no glyph for maps to `.notdef`, which in most
/// faces is a hollow box: printing one would be the renderer guessing out
/// loud. Taking no width and drawing nothing is what "this face cannot say
/// that" has looked like here since text was first drawn, and kerning is no
/// reason to change it.
///
/// **That is now the last resort rather than the first answer.** A character
/// this face lacks was handed to a face that has it before anything got here,
/// and reaches this line only when nothing in the chain covers it — which is
/// exactly the case `check` reports and the frame drops.
pub(super) fn shape(
    shaper: &Shaper<'_>,
    text: &str,
    scale: f32,
    face: usize,
    figures: Figures,
) -> Shaped {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    // Direction, script and language read off the characters themselves. A
    // title card is one language at a time, and guessing is what makes a
    // Cyrillic or Greek run get its own script's features rather than Latin's.
    buffer.guess_segment_properties();

    let shaped = shaper.shape(buffer, ShapeOptions::new().features(figures.features()));
    let mut glyphs = Vec::with_capacity(shaped.len());
    let mut pen = 0.0;
    for (glyph, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        if glyph.glyph_id == 0 {
            continue;
        }
        glyphs.push(Placed {
            id: GlyphId::new(glyph.glyph_id),
            at: (
                pen + position.x_offset as f32 * scale,
                position.y_offset as f32 * scale,
            ),
            face,
            cluster: glyph.cluster as usize,
        });
        pen += position.x_advance as f32 * scale;
    }
    Shaped { glyphs, width: pen }
}
