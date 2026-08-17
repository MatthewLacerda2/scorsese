//! Answering "is that a property at all, and if not, what did you mean?"
//!
//! The machinery only; the vocabulary belongs next to the code that implements
//! each property — [`crate::properties::ANIMATED`] here, and the mixer's own
//! list in `scorsese-render`, since `volume` is implemented by the mixer and a
//! name listed where nothing implements it is exactly the drift this is meant
//! to prevent. A [`Registry`] is however many of those lists a caller has.
//!
//! **Nothing here fails anything.** An unknown property is a warning: it is a
//! quality signal, not a correctness gate, and making it an error would turn
//! every newly animatable property into a breaking change for projects
//! authored against a build that had it.

use scorsese_core::PropertyPath;

use crate::distance;

/// One property something in this workspace can animate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Property {
    /// The dotted path a keyframe track names it by.
    pub path: &'static str,
    /// What animating it does, in words a report can print.
    pub describes: &'static str,
}

/// Every animatable property a caller knows about.
#[derive(Debug, Clone, Copy)]
pub struct Registry {
    sets: &'static [&'static [Property]],
}

impl Registry {
    /// A registry over the given vocabularies, in order.
    pub const fn new(sets: &'static [&'static [Property]]) -> Self {
        Self { sets }
    }

    /// Every property known, across every vocabulary.
    pub fn properties(&self) -> impl Iterator<Item = &'static Property> {
        self.sets.iter().copied().flatten()
    }

    pub(crate) fn get(&self, path: &PropertyPath) -> Option<&'static Property> {
        self.properties()
            .find(|property| property.path == path.as_str())
    }

    /// Whether anything animates this path — the yes or no that is all a
    /// caller deciding whether to warn ever needs.
    pub fn knows(&self, path: &PropertyPath) -> bool {
        self.get(path).is_some()
    }

    /// The known path an unknown one was probably meant to be.
    ///
    /// This is the point of the whole exercise — the case being caught is a
    /// typo, and "`opactiy` is not a property" is half an answer. Nothing is
    /// suggested unless one candidate is both close and closer than any other:
    /// a wrong suggestion is worse than none, because it is the one thing a
    /// reader will act on without checking.
    pub fn did_you_mean(&self, path: &PropertyPath) -> Option<&'static Property> {
        /// Two edits: a swapped pair of letters costs exactly this, and a
        /// transposition is the typo worth catching.
        const NEAREST: usize = 2;

        let mut ranked: Vec<(usize, &'static Property)> = self
            .properties()
            .map(|property| (distance::between(path.as_str(), property.path), property))
            .collect();
        ranked.sort_by_key(|(distance, property)| (*distance, property.path));
        let (best, property) = *ranked.first()?;
        // Far enough apart to be a different word, or two candidates equally
        // close: either way there is nothing to say with confidence.
        if best > NEAREST || best * 2 >= property.path.len() {
            return None;
        }
        match ranked.get(1) {
            Some((second, _)) if *second == best => None,
            _ => Some(property),
        }
    }
}
