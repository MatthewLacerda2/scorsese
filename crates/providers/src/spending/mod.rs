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
//!
//! # A retry is spent too
//!
//! Generation spending is read off the ledger of every brief sent ([`GENERATIONS_FILE`], [`history`]), not off
//! the assets: an asset carries what its *current* take cost, and a shot bought
//! twice would be counted once. The takes no asset points at any more are
//! still in the total, and the report names them on their own line
//! ([`Superseded`]), so a retry reads as a retry.

mod ledger;

use std::path::Path;

use scorsese_core::{AssetId, Project};

use crate::voices::design;

pub use ledger::{GENERATIONS_FILE, Sent, follow_renames, history};
pub(crate) use ledger::{designed, designs, readable, refused, sent, sent_all};

/// What a project has spent, by the kind of thing it was spent on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spending {
    /// What generating the assets is estimated to have cost — every shot,
    /// still, line and track sent, superseded takes included. Read off the
    /// ledger beside `project.json` ([`history`]), written as each was sent.
    pub generation_cents: u64,
    /// What designing voices is estimated to have cost — every design paid
    /// for, whether or not a voice was kept, and twice when it was paid for
    /// twice ([`design::spent`]).
    /// Not on the assets, because a designed voice is not an asset and never
    /// will be — it lives in an account somewhere else.
    pub design_cents: u64,
    /// The part of [`generation_cents`](Self::generation_cents) no asset
    /// shows any more.
    pub superseded: Superseded,
}

/// Takes that were paid for and then replaced — a shot bought again with a
/// new prompt, its first take still in `generated/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Superseded {
    /// How many.
    pub takes: usize,
    /// What they were worked out to cost, together.
    pub cents: u64,
}

impl Spending {
    /// Everything, which is what a ceiling is checked against.
    pub fn total(self) -> u64 {
        self.generation_cents.saturating_add(self.design_cents)
    }

    /// The report a person reads: the total, and the superseded takes in it
    /// on a line of their own when there are any.
    ///
    /// Assembled here so every surface that prints it says the same thing.
    pub fn says(self) -> String {
        let mut says = format!(
            "About {} spent on this project so far — our calculation, never a bill.",
            crate::prices::dollars(crate::prices::from_cents(self.total()), 2)
        );
        if let Some(line) = self.superseded.says() {
            says.push('\n');
            says.push_str(&line);
        }
        says
    }
}

impl Superseded {
    /// `N superseded takes, $X` — `None` when nothing was superseded.
    pub fn says(self) -> Option<String> {
        let plural = if self.takes == 1 { "" } else { "s" };
        (self.takes > 0).then(|| {
            format!(
                "{} superseded take{plural}, {}",
                self.takes,
                crate::prices::dollars(crate::prices::from_cents(self.cents), 2)
            )
        })
    }
}

/// What this project has spent so far.
///
/// Generations and voice designs, read together off the one ledger beside
/// `project.json` — and a caller that reads only one of them builds a ceiling
/// that is wrong by however much the other holds.
pub fn so_far(project: &Project, root: &Path) -> Spending {
    let sent = history(project, root);
    Spending {
        generation_cents: sent.iter().map(Sent::cents).sum(),
        design_cents: design::spent(root),
        superseded: superseded(project, &sent),
    }
}

/// The lines no asset accounts for.
///
/// Walking back from the newest, each asset accounts for at most two lines:
/// the newest whose brief names its current file (the take it shows), and,
/// while it waits on a ticket, its newest line of all (the take in flight).
/// Every other line — an older take, a brief sent twice, an asset since
/// removed, a generation lost after it was sent — is superseded. A refused
/// line is none of these: it was never charged.
fn superseded(project: &Project, sent: &[Sent]) -> Superseded {
    let mut shown: Vec<&AssetId> = Vec::new();
    let mut waited: Vec<&AssetId> = Vec::new();
    let mut superseded = Superseded::default();
    for line in sent.iter().rev().filter(|line| !line.refused) {
        let kept = project.asset(&line.asset).is_some_and(|asset| {
            let first = !shown.contains(&&asset.id) && !waited.contains(&&asset.id);
            let current = asset
                .path
                .as_ref()
                .and_then(|path| ledger::take(&asset.id, path))
                .is_some_and(|take| take == line.brief);
            if current && !shown.contains(&&asset.id) {
                shown.push(&asset.id);
                true
            } else if first && asset.operation.is_some() {
                waited.push(&asset.id);
                true
            } else {
                false
            }
        });
        if !kept {
            superseded.takes += 1;
            superseded.cents += line.estimated_cost_cents;
        }
    }
    superseded
}
