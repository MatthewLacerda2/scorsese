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

/// Where the files sit, relative to this source file.
macro_rules! face {
    ($file:literal) => {
        include_bytes!(concat!("../../../fonts/", $file))
    };
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
    /// How its weights are made.
    pub cut: Cut,
}

/// Every face this build ships, in the order a list of them should read:
/// the two defaults first, then sans, serif, display, mono.
pub const SHIPPED: &[Family] = &[
    Family {
        name: "inter",
        aliases: &["sans"],
        family: "Inter",
        cut: Cut::Variable(face!("Inter-V.ttf")),
    },
    Family {
        name: "source-serif",
        aliases: &["serif"],
        family: "Source Serif 4",
        cut: Cut::Variable(face!("SourceSerif4Variable-Roman.ttf")),
    },
    Family {
        name: "liberation-sans",
        aliases: &[],
        family: "Liberation Sans",
        cut: Cut::Drawn(&[
            (400, face!("LiberationSans-Regular.ttf")),
            (700, face!("LiberationSans-Bold.ttf")),
        ]),
    },
    Family {
        name: "liberation-serif",
        aliases: &[],
        family: "Liberation Serif",
        cut: Cut::Drawn(&[
            (400, face!("LiberationSerif-Regular.ttf")),
            (700, face!("LiberationSerif-Bold.ttf")),
        ]),
    },
    Family {
        name: "montserrat",
        aliases: &[],
        family: "Montserrat",
        cut: Cut::Variable(face!("Montserrat[wght].ttf")),
    },
    Family {
        name: "lora",
        aliases: &[],
        family: "Lora",
        cut: Cut::Variable(face!("Lora[wght].ttf")),
    },
    Family {
        name: "playfair-display",
        aliases: &[],
        family: "Playfair Display",
        cut: Cut::Variable(face!("PlayfairDisplay[wght].ttf")),
    },
    Family {
        name: "jetbrains-mono",
        aliases: &[],
        family: "JetBrains Mono",
        cut: Cut::Variable(face!("JetBrainsMono[wght].ttf")),
    },
];

impl Family {
    /// Whether `wanted` names this face, by its own name or by an alias.
    fn answers_to(&self, wanted: &str) -> bool {
        self.name == wanted || self.aliases.contains(&wanted)
    }

    /// The weights a drawn family was drawn at, for a message that has to say
    /// what to write instead. Empty for a variable family, whose answer is a
    /// range rather than a list.
    pub fn drawn_weights(&self) -> Vec<u16> {
        match self.cut {
            Cut::Variable(_) => Vec::new(),
            Cut::Drawn(files) => files.iter().map(|(weight, _)| *weight).collect(),
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
