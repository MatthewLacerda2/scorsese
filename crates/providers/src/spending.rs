//! What a project has already spent, and on what.
//!
//! One question — *how much has this project cost so far?* — with one answer,
//! because it is asked in two different moods and the two must not disagree. A
//! spending ceiling asks it to decide whether the next call may go ahead; a
//! person asks it to decide whether they want to. The first person to notice
//! two answers differ would be the person who had already paid.
//!
//! # Why it is a breakdown rather than a number
//!
//! **A ceiling counts every cent; a report separates them.** Money spent
//! designing a voice is money, so it is under the ceiling exactly as a
//! generated shot is. But it is *not* what the shots cost, and folding the two
//! together produces a generation total that moves while nobody is generating —
//! which is a counter nobody trusts, and the mistrust is deserved, because the
//! number really is describing something other than what it is labelled.
//!
//! So [`Spending`] keeps the lines apart and [`Spending::total`] adds them up,
//! and the shape makes the right thing the easy thing at both call sites.
//!
//! # None of these is a bill
//!
//! Every figure here is scorsese's own arithmetic over a rate table somebody
//! copied off a vendor's page — no provider reports what a call cost. That is
//! why the asset field is `estimated_cost_cents` and why nothing here is
//! spelled shorter. See [`prices`](crate::prices).

use std::path::Path;

use scorsese_core::Project;

use crate::voices::design;

/// What a project has spent, by the kind of thing it was spent on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spending {
    /// What generating the assets is estimated to have cost — the shots, and
    /// the narration once there is any. Read off the document, because that is
    /// where each generation wrote what it was estimated to cost at the moment
    /// it happened.
    pub generation_cents: u64,
    /// What designing voices is estimated to have cost. Read off the ledger
    /// beside `project.json`, because a designed voice is not an asset and
    /// never will be — it lives in an account somewhere else.
    pub design_cents: u64,
}

impl Spending {
    /// Everything, which is what a ceiling is checked against.
    pub fn total(self) -> u64 {
        self.generation_cents.saturating_add(self.design_cents)
    }
}

/// What this project has spent so far.
///
/// The document and the ledger, read together — the two places money leaves a
/// trace, and a caller that reads only one of them builds a ceiling that is
/// wrong by however much the other holds.
pub fn so_far(project: &Project, root: &Path) -> Spending {
    Spending {
        generation_cents: project
            .assets
            .iter()
            .filter_map(|asset| asset.estimated_cost_cents)
            .sum(),
        design_cents: design::spent(root),
    }
}
