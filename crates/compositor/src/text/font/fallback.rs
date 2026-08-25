//! The faces tried when the one a document named cannot say something.
//!
//! **An ordered list, and exactly one entry is shipped.** Noto Color Emoji, in
//! its COLRv1 vector build — see `crates/compositor/fonts/README.md` for which
//! release and why the vector build rather than the bitmap one. Nothing names
//! it and nothing can: a `style` chooses the face a caption is *set* in, and a
//! fallback is not that. It is what happens when the chosen face has no glyph.
//!
//! It is a list rather than a second face because the shape is the point. Eight
//! families ship, none of them covers Unicode, and the next gap somebody hits
//! will be a script rather than an emoji — a CJK face belongs at the end of
//! [`chain`] and nowhere else. Written as *the emoji face*, adding one would be
//! a rewrite; written as a chain, it is a line.
//!
//! **Order is the whole of the policy.** The face the document named is always
//! tried first, so a character it can draw is drawn by it — a text face that
//! has `☺` sets `☺` in text, not in colour. Only what it cannot say reaches
//! this list, and within the list the first face that can say something wins.

use std::sync::OnceLock;

use super::{Face, Font};
use crate::text::runs;
use crate::text::shape::Shaped;

/// The faces to try, in order, after the one a document named.
///
/// Read once for the process. The bytes are compiled in, so a failure here
/// would mean a corrupt binary rather than a bad project.
fn chain() -> &'static [Font] {
    static CHAIN: OnceLock<Vec<Font>> = OnceLock::new();
    CHAIN.get_or_init(|| {
        vec![
            Font::from_bytes(include_bytes!("../../../fonts/Noto-COLRv1.ttf"), None)
                .expect("the emoji face compiled into this binary parses"),
        ]
    })
}

/// Whether any face in the chain has a glyph for `character`.
///
/// What [`Font::uncovered`] asks after the named face has said no. A character
/// answered here is drawn, so it is not missing from the frame and `check` has
/// nothing to report about it.
pub(super) fn covers(character: char) -> bool {
    chain().iter().any(|font| font.has(character))
}

/// A chain of faces at one size: the one a document named, then the fallbacks.
///
/// This is what measuring and drawing are handed, and the reason they are
/// handed it rather than a single face is that the two must never disagree.
/// A line's width is the sum of its runs' widths, and a run's width comes from
/// the face that will actually draw it — so wrapping asks the same faces the
/// pen will.
pub(in crate::text) struct Faces<'a> {
    /// Index `0` is the named face. Never empty.
    faces: Vec<Face<'a>>,
}

impl<'a> Faces<'a> {
    /// The chain `named` heads, every face taken at `size`.
    pub(super) fn of(named: &'a Font, size: f32) -> Self {
        // `&'static [Font]` shortened to `&'a [Font]` deliberately, so every
        // face in the vector is built at one lifetime rather than relying on a
        // borrowed view of a face being covariant in it.
        let fallbacks: &'a [Font] = chain();
        let mut faces = Vec::with_capacity(1 + fallbacks.len());
        faces.push(named.at(size));
        faces.extend(fallbacks.iter().map(|font| font.at(size)));
        Self { faces }
    }

    /// Which glyphs set `text` and where each one goes, kerning applied — split
    /// into runs by which face can say what, and drawn end to end.
    ///
    /// The only way to a width in this module, exactly as a single face's
    /// [`Face::shape`] was: a line measured any other way would be wrapped at a
    /// place the drawn text does not break.
    pub(in crate::text) fn shape(&self, text: &str) -> Shaped {
        let mut whole = Shaped::default();
        for run in runs::split(text, self.faces.len(), |face, character| {
            self.faces[face].covers(character)
        }) {
            let face = &self.faces[run.face];
            whole.append(face.shape(&text[run.range], run.face));
        }
        whole
    }

    /// How far the tallest glyph reaches above the baseline and the lowest
    /// below it — **from the named face, always**.
    ///
    /// A fallback supplies its own glyphs' advances and nothing else. Noto
    /// Color Emoji sets much taller than Inter does, so a caption whose extents
    /// came off whichever faces happened to be in it would grow a line the
    /// moment somebody typed an emoji into it and shrink again when they
    /// deleted one. The block is the named face's block; the emoji is a guest
    /// in it.
    pub(in crate::text) fn extents(&self) -> (f32, f32) {
        self.named().extents()
    }

    /// The face a placed glyph came from, for drawing it.
    pub(in crate::text) fn face(&self, index: usize) -> &Face<'a> {
        // A glyph carries the index it was shaped with, so this is in range for
        // anything this chain produced. Clamped rather than trusted anyway,
        // because the alternative is a panic in the middle of a render.
        &self.faces[index.min(self.faces.len() - 1)]
    }

    fn named(&self) -> &Face<'a> {
        &self.faces[0]
    }
}
