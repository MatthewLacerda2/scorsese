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
use scorsese_core::spelling::closest;

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

    /// The known path an unknown one was probably meant to be, by
    /// [`scorsese_core::spelling::closest`].
    ///
    /// This is the point of the whole exercise — the case being caught is a
    /// typo, and "`opactiy` is not a property" is half an answer.
    pub fn did_you_mean(&self, path: &PropertyPath) -> Option<&'static Property> {
        let meant = closest(
            path.as_str(),
            self.properties().map(|property| property.path),
        )?;
        self.properties().find(|property| property.path == meant)
    }
}
