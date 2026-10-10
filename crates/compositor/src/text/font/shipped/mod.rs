//! The faces scorsese ships, and which files make each one.
//!
//! **The catalogue lives here and nowhere else.** `scorsese-core` says a font
//! is a name or a file, which is a property type; which names exist is a set of
//! property *values*, and values belong next to the code that implements them.
//! That is the same arrangement [`crate::properties::ANIMATED`] has, for the
//! same reason: adding a face is a change in this file, beside the
//! `include_bytes!` that makes it real, and never a change to the format.
//!
//! ## A face is a table, not a file
//!
//! Two shapes, and the difference is not cosmetic:
//!
//! - a **variable** family is one file whose `wght` axis covers a range, and
//!   any weight inside it is a real position on that axis;
//! - a **drawn** family is several files the designer actually drew, and the
//!   only weights it has are the ones in the table.
//!
//! Liberation is the second kind — there is no variable build of it anywhere,
//! upstream ships Regular and Bold as separate files — and it is the reason
//! this distinction exists at all. Handling only the first kind is what made
//! the Arial and Times look-alikes unshippable once `weight` arrived.
//!
//! **A drawn family refuses a weight it was not drawn at**, naming the ones it
//! has, rather than snapping to the nearest. Snapping is the same silent
//! substitution the variable rules already refuse when a weight falls off the
//! end of an axis, and being wrong quietly about which weight you got is worse
//! than being told to write 700 instead of 600.
//!
//! ## Italic is a second table, not an effect
//!
//! Every family here carries a whole second set of files for its italic,
//! because **a real italic is a different drawing** — different letterforms,
//! not the upright ones leaned over. Inter makes the point on its own: its
//! `Inter-V.ttf` has a `slnt` axis, which produces an *oblique*, and the
//! separate `Inter-italic.ttf` is the drawn italic with the letters actually
//! redrawn. The drawn one is what `italic: true` reaches, every time.
//!
//! A family with no italic is [`Family::italic`] of `None`, and asking for one
//! is refused rather than faked. Most display, handwriting and script faces are
//! that — Anton, Bangers and Pacifico were drawn upright and nothing else — and
//! `italic: true` on one of them is an error naming the family, never a shear.

/// Where the files sit, relative to this source file.
macro_rules! face {
    ($file:literal) => {
        include_bytes!(concat!("../../../../fonts/", $file))
    };
}

/// Upright or italic — which of a family's two tables to read.
///
/// A named pair rather than a `bool`, so a call site says which it means. A
/// third argument spelled `true` is a thing nobody can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Slant {
    /// The upright drawing.
    #[default]
    Upright,
    /// The drawn italic, when the family has one.
    Italic,
}

/// How a family's weights are made.
#[derive(Debug, Clone, Copy)]
pub enum Cut {
    /// One file, whose `wght` axis is asked for the weight. What the axis will
    /// not reach, the face refuses.
    Variable(&'static [u8]),
    /// Files the designer drew, by the weight each one is. Nothing between
    /// them exists.
    Drawn(&'static [(u16, &'static [u8])]),
}

/// One family a document can name, and the files that make its weights.
#[derive(Debug, Clone, Copy)]
pub struct Family {
    /// What a `style` writes to get it.
    pub name: &'static str,
    /// Other names that mean this face. `sans` and `serif` are here rather
    /// than being faces of their own, so that every document ever written goes
    /// on meaning what it meant while the thing they point at stays free to
    /// change.
    pub aliases: &'static [&'static str],
    /// The family's own name, for a message a person reads.
    pub family: &'static str,
    /// How its upright weights are made.
    pub cut: Cut,
    /// The same, for its italic, when the family has one drawn.
    ///
    /// `None` is a family with no italic, and asking for one is an error. It is
    /// never a licence to slant the upright: an oblique is a different thing
    /// and looks like one.
    pub italic: Option<Cut>,
}

mod display;
mod hand;
mod rounded;
mod sans;
mod serif;

/// Every face this build ships, in the order a list of them should read: the
/// two defaults first, then sans, serif, rounded, display (monospace among
/// them), and handwriting and script last.
///
/// One slice, assembled at compile time from a file per kind, because a
/// catalogue of forty families does not fit one file and its readers — the
/// lookup below, the page's font declarations — want one list, not six.
pub const SHIPPED: &[Family] = &ALL;

const ALL: [Family; COUNT] = concat(&[
    &DEFAULTS,
    &sans::FAMILIES,
    &serif::FAMILIES,
    &rounded::FAMILIES,
    &display::FAMILIES,
    &hand::FAMILIES,
]);

const COUNT: usize = DEFAULTS.len()
    + sans::FAMILIES.len()
    + serif::FAMILIES.len()
    + rounded::FAMILIES.len()
    + display::FAMILIES.len()
    + hand::FAMILIES.len();

/// What `sans` and `serif` mean. Kept here, above the kinds, because every
/// document written before the list grew names one of these two.
const DEFAULTS: [Family; 2] = [
    Family {
        name: "inter",
        aliases: &["sans"],
        family: "Inter",
        cut: Cut::Variable(face!("Inter-V.ttf")),
        italic: Some(Cut::Variable(face!("Inter-Italic.ttf"))),
    },
    Family {
        name: "source-serif",
        aliases: &["serif"],
        family: "Source Serif 4",
        cut: Cut::Variable(face!("SourceSerif4Variable-Roman.ttf")),
        italic: Some(Cut::Variable(face!("SourceSerif4Variable-Italic.ttf"))),
    },
];

/// The kinds' lists laid end to end. `N` is checked against what arrived, so
/// a family added to a kind without [`COUNT`] seeing it fails the build.
const fn concat<const N: usize>(parts: &[&[Family]]) -> [Family; N] {
    let mut all = [DEFAULTS[0]; N];
    let (mut at, mut part) = (0, 0);
    while part < parts.len() {
        let mut i = 0;
        while i < parts[part].len() {
            all[at] = parts[part][i];
            (at, i) = (at + 1, i + 1);
        }
        part += 1;
    }
    assert!(at == N, "COUNT disagrees with the kinds' lists");
    all
}

impl Family {
    /// A variable family with no alias: one file per slant, the `wght` axis
    /// answering for every weight. `italic` is `None` where none is drawn.
    const fn variable(
        name: &'static str,
        family: &'static str,
        upright: &'static [u8],
        italic: Option<&'static [u8]>,
    ) -> Self {
        Self {
            name,
            aliases: &[],
            family,
            cut: Cut::Variable(upright),
            italic: match italic {
                Some(bytes) => Some(Cut::Variable(bytes)),
                None => None,
            },
        }
    }

    /// A drawn family with no alias, from its tables. A display or script
    /// face drawn once, Regular and upright, is `&[(400, file)]` and `None`.
    const fn drawn(
        name: &'static str,
        family: &'static str,
        upright: &'static [(u16, &'static [u8])],
        italic: Option<&'static [(u16, &'static [u8])]>,
    ) -> Self {
        Self {
            name,
            aliases: &[],
            family,
            cut: Cut::Drawn(upright),
            italic: match italic {
                Some(files) => Some(Cut::Drawn(files)),
                None => None,
            },
        }
    }
}

impl Family {
    /// Whether `wanted` names this face, by its own name or by an alias.
    fn answers_to(&self, wanted: &str) -> bool {
        self.name == wanted || self.aliases.contains(&wanted)
    }

    /// The table for one slant, or `None` when the family has no italic drawn.
    pub fn table(&self, slant: Slant) -> Option<Cut> {
        match slant {
            Slant::Upright => Some(self.cut),
            Slant::Italic => self.italic,
        }
    }

    /// The weights a drawn family was drawn at, for a message that has to say
    /// what to write instead. Empty for a variable family, whose answer is a
    /// range rather than a list.
    pub fn drawn_weights(&self, slant: Slant) -> Vec<u16> {
        match self.table(slant) {
            Some(Cut::Drawn(files)) => files.iter().map(|(weight, _)| *weight).collect(),
            _ => Vec::new(),
        }
    }
}

/// The family a document's `font` names, if this build ships one.
pub fn family(name: &str) -> Option<&'static Family> {
    SHIPPED.iter().find(|family| family.answers_to(name))
}

/// Every name a document may write, aliases included, in listing order.
///
/// Published because "which fonts are there?" is the question being asked
/// whenever somebody gets a name wrong, and a refusal that does not answer it
/// is half a refusal.
pub fn names() -> impl Iterator<Item = &'static str> {
    SHIPPED
        .iter()
        .flat_map(|family| std::iter::once(family.name).chain(family.aliases.iter().copied()))
}
