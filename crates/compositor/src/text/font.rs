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
//!
//! **Weight is the one exception, and it is here because silence was the bug.**
//! Most modern open faces now ship *only* as variable files — one file holding
//! a continuous `wght` axis — and every one of them declares a default instance
//! that is whatever the designer chose. Manrope's is 200; Outfit's is 100.
//! Reading such a file and drawing with `LocationRef::default()` sets a title
//! card in hairline Thin and reports nothing, which is a shot rendered wrong
//! that no error mentions. So the axis is not something this module may ignore:
//! [`Font::from_bytes`] takes the weight alongside the bytes, and the
//! combinations that cannot mean anything are refused rather than guessed at.
//! Only `wght` is read — `opsz`, `wdth` and `slnt` are real axes and none of
//! them is what "make this bold" means.

use std::fmt;
use std::sync::OnceLock;

use skrifa::instance::{Location, LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::outline::{DrawSettings, OutlineGlyphCollection, OutlinePen};
use skrifa::{FontRef, MetadataProvider, Tag};

/// The variation axis that means "how heavy", as OpenType spells it.
const WEIGHT_AXIS: Tag = Tag::new(b"wght");

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
    /// Where in variation space to draw. Empty for a static file, and one
    /// normalised coordinate per `fvar` axis for a variable one — computed
    /// once when the face is made, because it is the same for every glyph.
    location: Location,
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
    /// face, as bytes — set at `weight` if it is variable.
    ///
    /// Bytes rather than a path: this crate does no file I/O, so opening the
    /// file belongs to whoever knows where the project root is. Parsing here
    /// rather than at the first glyph means a bad file is a refusal before a
    /// render starts instead of a panic part way through one.
    ///
    /// **Weight and file have to agree, and disagreeing is an error rather
    /// than a preference.** A variable file with no weight named is refused,
    /// because the alternative is drawing at a default the document never
    /// chose and never mentioned — the file's `fvar` default is 200 in Manrope
    /// and 100 in Outfit, so "it will be Regular" is not a safe guess anywhere.
    /// A weight the axis does not reach is refused with the range it does
    /// reach, since clamping 900 to 800 would be a second silent substitution.
    /// A static file with a weight named is refused too: it has exactly one
    /// weight, and quietly ignoring the field is how someone comes to insist
    /// their bold is broken.
    pub fn from_bytes(bytes: &[u8], weight: Option<u16>) -> Result<Self, FontError> {
        let font = FontRef::new(bytes).map_err(|error| FontError::Unreadable(error.to_string()))?;
        let location = locate(&font, weight)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            location,
        })
    }

    /// The face set at one size, which is the form everything else here wants.
    pub(super) fn at(&self, size: f32) -> Face<'_> {
        let font = FontRef::new(&self.bytes).expect("these bytes parsed when the font was made");
        let at = Size::new(size);
        let location = LocationRef::from(&self.location);
        let line = font.metrics(at, location);
        Face {
            charmap: font.charmap(),
            glyphs: font.outline_glyphs(),
            metrics: font.glyph_metrics(at, location),
            ascent: line.ascent,
            descent: line.descent,
            at,
            location: self.location.clone(),
        }
    }

    /// Reads a face this build ships. Its bytes are compiled in, so failing
    /// here would mean a corrupt binary rather than a bad project.
    fn shipped(bytes: &[u8]) -> Self {
        Self::from_bytes(bytes, None).expect("a font compiled into this binary parses")
    }
}

/// Turns a weight into a position in the file's variation space, refusing
/// every pairing of file and weight that cannot mean one thing.
///
/// A file with no `wght` axis is static as far as this is concerned — a face
/// varying only on `opsz` or `wdth` has one weight like any static one, and
/// naming a weight for it is the same mistake.
fn locate(font: &FontRef<'_>, weight: Option<u16>) -> Result<Location, FontError> {
    let axes = font.axes();
    let axis = axes.get_by_tag(WEIGHT_AXIS);
    match (axis, weight) {
        (None, None) => Ok(Location::default()),
        (None, Some(weight)) => Err(FontError::StaticWithWeight { weight }),
        (Some(axis), None) => Err(FontError::VariableWithoutWeight {
            min: axis.min_value(),
            default: axis.default_value(),
            max: axis.max_value(),
        }),
        (Some(axis), Some(weight)) => {
            let asked = f32::from(weight);
            if asked < axis.min_value() || asked > axis.max_value() {
                return Err(FontError::WeightOffAxis {
                    weight,
                    min: axis.min_value(),
                    max: axis.max_value(),
                });
            }
            Ok(axes.location([(WEIGHT_AXIS, asked)]))
        }
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
    /// Cloned from the face rather than borrowed, so drawing a glyph needs
    /// nothing but this struct. It is a handful of coordinates.
    location: Location,
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
        let _ = glyph.draw(
            DrawSettings::unhinted(self.at, LocationRef::from(&self.location)),
            pen,
        );
    }
}

/// Why a font file could not be used.
///
/// Not `Eq`: an axis states its bounds as the floats the `fvar` table holds,
/// and rounding them to say they compare would be inventing a fact about the
/// file.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FontError {
    /// Not a font this build can read: the wrong kind of file, a truncated
    /// one, or a format the parser does not know.
    #[error("this is not a font scorsese can read: {0}")]
    Unreadable(
        /// What the parser said, in its own words.
        String,
    ),

    /// A variable font, and nothing said about how heavy to set it.
    ///
    /// The message names the file's own default because that is the weight
    /// that *would* have been used, and it is the number a reader needs in
    /// order to see the trap: a file whose default is 200 sets a title in
    /// hairline Thin while looking entirely correct.
    #[error(
        "this font is variable — it has a `wght` axis from {min} to {max}, defaulting to \
         {default} — so `style.weight` has to say which weight to set it at"
    )]
    VariableWithoutWeight {
        /// The lightest weight the file offers.
        min: f32,
        /// The instance the file would fall back to, which is exactly what is
        /// not being allowed to happen silently.
        default: f32,
        /// The heaviest weight the file offers.
        max: f32,
    },

    /// A weight this file's axis does not reach. Refused rather than clamped:
    /// clamping 900 to 800 is the same silent substitution the whole rule
    /// exists to stop, one step further along.
    #[error("this font's `wght` axis runs from {min} to {max}, so it has no weight {weight}")]
    WeightOffAxis {
        /// The weight asked for.
        weight: u16,
        /// The lightest weight the file offers.
        min: f32,
        /// The heaviest weight the file offers.
        max: f32,
    },

    /// A weight named for a file that has only one. Ignoring it would look
    /// exactly like honouring it.
    ///
    /// The message says "no `wght` axis" rather than "static", because that is
    /// the fact and it is very slightly narrower: a face varying only on
    /// `opsz` or `wdth` is a variable file with one weight, and telling its
    /// author it is static would send them looking for the wrong thing.
    #[error("this font has no `wght` axis — it has one weight — so it cannot be set at {weight}")]
    StaticWithWeight {
        /// The weight that could not be honoured.
        weight: u16,
    },
}
