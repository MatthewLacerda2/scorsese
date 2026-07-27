//! The faces that can be drawn with, and the two that ship.
//!
//! **The fonts are committed to the repository**, in `crates/compositor/fonts/`
//! with the SIL Open Font License beside them. That is forced rather than
//! preferred: a system-font lookup resolves to a different file on Linux, macOS
//! and Windows, so a golden reference blessed on one machine would fail on
//! another and the pixel gate would become noise. Deterministic text means a
//! font we ship.
//!
//! Liberation Sans and Liberation Serif are *metric-compatible* with Arial and
//! Times New Roman — the same advance widths, so text laid out in one breaks
//! lines in the same places as the other. The names an author reaches for are
//! proprietary and cannot be committed; these are the well-trodden stand-ins.
//! They are defaults, not the vocabulary: [`Font::from_bytes`] takes any font
//! file a project brings with it.
//!
//! What is read out of a face is deliberately narrow — a glyph for a character,
//! its advance, and the outline to fill. Kerning and the rest of shaping are a
//! different problem with a different crate behind it, and a title card does
//! not have it.

use std::fmt;
use std::sync::OnceLock;

use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::outline::{DrawSettings, OutlineGlyphCollection, OutlinePen};
use skrifa::{FontRef, MetadataProvider};

/// Liberation Sans, regular weight. One weight of each face rather than a
/// family: bold and italic are a real feature with a real vocabulary
/// (`weight`, `slant`), and shipping four files against a `style` nothing can
/// select would be a megabyte pretending to be a choice.
const SANS: &[u8] = include_bytes!("../../fonts/LiberationSans-Regular.ttf");

/// Liberation Serif, regular weight, under the same rule.
const SERIF: &[u8] = include_bytes!("../../fonts/LiberationSerif-Regular.ttf");

/// A face, checked and ready to draw with.
///
/// Holds the file rather than a parsed view of it: the parsed view borrows the
/// bytes, and a struct holding both would be self-referential. Re-reading the
/// table directory is a handful of bounds checks, paid once per drawn block
/// rather than per glyph.
pub struct Font {
    bytes: Vec<u8>,
}

impl Font {
    /// The shipped sans face, read once for the process.
    pub fn sans() -> &'static Self {
        static FONT: OnceLock<Font> = OnceLock::new();
        FONT.get_or_init(|| Self::shipped(SANS))
    }

    /// The shipped serif face, under the same rule as [`Font::sans`].
    pub fn serif() -> &'static Self {
        static FONT: OnceLock<Font> = OnceLock::new();
        FONT.get_or_init(|| Self::shipped(SERIF))
    }

    /// Parses a font file a project brought with it — a TrueType or OpenType
    /// face, as bytes.
    ///
    /// Bytes rather than a path: this crate does no file I/O, so opening the
    /// file belongs to whoever knows where the project root is. Parsing here
    /// rather than at the first glyph means a bad file is a refusal before a
    /// render starts instead of a panic part way through one.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FontError> {
        FontRef::new(bytes).map_err(|error| FontError::Unreadable(error.to_string()))?;
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }

    /// The face set at one size, which is the form everything else here wants.
    pub(super) fn at(&self, size: f32) -> Face<'_> {
        let font = FontRef::new(&self.bytes).expect("these bytes parsed when the font was made");
        let at = Size::new(size);
        let line = font.metrics(at, LocationRef::default());
        Face {
            charmap: font.charmap(),
            glyphs: font.outline_glyphs(),
            metrics: font.glyph_metrics(at, LocationRef::default()),
            ascent: line.ascent,
            descent: line.descent,
            at,
        }
    }

    /// Reads a face this build ships. Its bytes are compiled in, so failing
    /// here would mean a corrupt binary rather than a bad project.
    fn shipped(bytes: &[u8]) -> Self {
        Self::from_bytes(bytes).expect("a font compiled into this binary parses")
    }
}

impl fmt::Debug for Font {
    /// The file itself is hundreds of kilobytes and nothing a reader wants in
    /// a log line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font").finish_non_exhaustive()
    }
}

/// One face at one size: everything measuring and drawing needs, looked up
/// once instead of per character.
pub(super) struct Face<'a> {
    charmap: skrifa::charmap::Charmap<'a>,
    glyphs: OutlineGlyphCollection<'a>,
    metrics: GlyphMetrics<'a>,
    ascent: f32,
    descent: f32,
    at: Size,
}

impl Face<'_> {
    /// How far the pen moves to set `character`.
    ///
    /// A character the face has no glyph for advances by nothing rather than
    /// by a guess, so a string of them takes no width and draws nothing —
    /// which is what "this face cannot say that" should look like.
    pub(super) fn advance(&self, character: char) -> f32 {
        self.charmap
            .map(character)
            .and_then(|glyph| self.metrics.advance_width(glyph))
            .unwrap_or(0.0)
    }

    /// How far the tallest glyph reaches above the baseline and the lowest
    /// below it. The descent is negative, as the face states it.
    pub(super) fn extents(&self) -> (f32, f32) {
        (self.ascent, self.descent)
    }

    /// Walks `character`'s outline through `pen`, in font-space with **y
    /// upwards** — the convention outlines are written in, which whoever draws
    /// them has to flip onto a raster.
    pub(super) fn outline(&self, character: char, pen: &mut impl OutlinePen) {
        let Some(glyph) = self
            .charmap
            .map(character)
            .and_then(|id| self.glyphs.get(id))
        else {
            return;
        };
        // A glyph that will not draw — a corrupt outline in an otherwise
        // readable file — leaves a gap of its own width rather than stopping
        // the render. The rest of the title is still worth having.
        let _ = glyph.draw(DrawSettings::unhinted(self.at, LocationRef::default()), pen);
    }
}

/// Why a font file could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FontError {
    /// Not a font this build can read: the wrong kind of file, a truncated
    /// one, or a format the parser does not know.
    #[error("this is not a font scorsese can read: {0}")]
    Unreadable(
        /// What the parser said, in its own words.
        String,
    ),
}
