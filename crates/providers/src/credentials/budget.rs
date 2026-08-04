//! The ceiling on what one run may spend.

/// What a project is allowed to spend, and what it has spent already.
///
/// Money in whole US cents throughout. Cents rather than dollars because a
/// total is a sum over a table and a sum of floats is not the same number twice
/// — and a ceiling that is off by a rounding error is a ceiling nobody can
/// reason about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    ceiling: Option<u64>,
    spent: u64,
}

/// A run that would cross the ceiling.
///
/// Carries every number rather than only the verdict, because "refused" on its
/// own leaves somebody to work out how much room they actually have — and the
/// point of stopping here is to hand back a decision, not a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "this run would spend {estimate} cents on top of {spent} already spent, \
     which is over the {ceiling}-cent ceiling by {over} — \
     raise the ceiling in settings, or generate fewer shots"
)]
pub struct OverBudget {
    /// What the run was estimated to cost.
    pub estimate: u64,
    /// What has been spent before it.
    pub spent: u64,
    /// The ceiling it would cross.
    pub ceiling: u64,
    /// By how much.
    pub over: u64,
}

impl Budget {
    /// A budget with a ceiling.
    pub fn new(ceiling: u64, spent: u64) -> Self {
        Self {
            ceiling: Some(ceiling),
            spent,
        }
    }

    /// A budget with no ceiling — which is what a fresh install has.
    ///
    /// Deliberately not a default number. A limit nobody chose is not a limit,
    /// and inventing one would refuse the first honest run somebody made.
    pub fn unlimited(spent: u64) -> Self {
        Self {
            ceiling: None,
            spent,
        }
    }

    /// The budget a settings file describes, against what a project has spent.
    pub fn from_settings(settings: &super::Settings, spent: u64) -> Self {
        Self {
            ceiling: settings.budget_cents,
            spent,
        }
    }

    /// What is left before the ceiling, or `None` when there is not one.
    pub fn remaining(self) -> Option<u64> {
        self.ceiling
            .map(|ceiling| ceiling.saturating_sub(self.spent))
    }

    /// Whether a run of this size may go ahead.
    ///
    /// **This answer is not negotiable by a flag.** `--yes` says nobody is
    /// there to be asked, which is exactly the situation the ceiling exists
    /// for: an agent working overnight cannot be asked a question, so it is
    /// given a number it may not cross. A limit a flag can wave through is not
    /// a limit.
    pub fn check(self, estimate: u64) -> Result<(), OverBudget> {
        let Some(ceiling) = self.ceiling else {
            return Ok(());
        };
        let total = self.spent.saturating_add(estimate);
        if total <= ceiling {
            return Ok(());
        }
        Err(OverBudget {
            estimate,
            spent: self.spent,
            ceiling,
            over: total - ceiling,
        })
    }
}
