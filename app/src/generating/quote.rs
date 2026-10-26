//! What a run would cost, read out of the document before anyone commits to it.
//!
//! **Every figure here is an estimate, and the words on screen say so.** No
//! provider scorsese talks to reports what a generation cost — a finished Veo
//! operation carries a video link and nothing else — so a project total is our
//! own arithmetic over a published rate table. A window is the one place a
//! person reads that number before deciding to spend more, which makes it the
//! place it must not read as a receipt. See `docs/prices.md`.
//!
//! # The library's quote, never a copy of it
//!
//! The items are [`quote::generation`]'s — the one list `scorsese generate`
//! prints and the MCP tool answers with — and what has been spent is
//! [`spending::so_far`]'s. This window used to keep its own copies of both, and
//! they drifted the way copies do (#1104): its quote skipped any asset whose
//! `state` said `generated`, while the run decides by the **brief hash**, so a
//! shot whose prompt was edited in this very window was priced at nothing and
//! then paid for; and its spend left out designed voices, so its ceiling was
//! lower than the one the run enforces. Asking the library is what makes the
//! number on screen the number GO spends.
//!
//! # Grouped by vendor, never by one filter
//!
//! Shots, stills and lines are grouped by **asset kind**, because their rates
//! are orders of magnitude apart. Quoting everything merely *prompted* as a
//! shot is what once priced a line of narration as an eight-second 1080p
//! shot — ninety-six cents for something that costs two.
//! `narration_is_not_quoted_as_a_video_shot` is that bug, kept.

use std::path::Path;

use scorsese_core::{AssetKind, Project};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::prices::{dollars, from_cents};
use scorsese_providers::quote::{self, Item, Unquotable};
use scorsese_providers::spending::{self, Superseded};

/// What a run would cost, and what room there is for it.
#[derive(Clone)]
pub(crate) struct Quote {
    /// The library's quote, or why it cannot price this project.
    priced: Result<quote::Quote, Unquotable>,
    /// What the project's ledgers already say has been spent — generations,
    /// superseded takes included, and designed voices.
    pub(crate) spent_cents: u64,
    /// The takes in that figure no asset shows any more.
    pub(crate) superseded: Superseded,
    /// The ceiling, when somebody set one. Set again on every frame from the
    /// window's settings, since the dialog's own field changes it.
    pub(crate) ceiling_cents: Option<u64>,
}

impl Quote {
    /// Reads what this project would cost to realise, against the ceiling in
    /// `settings`.
    ///
    /// The ceiling is **handed in** rather than loaded here, and that is what
    /// makes this quote reproducible. `Settings::load` reads a file belonging
    /// to whoever is at the machine, so a quote that called it drew a different
    /// sentence for every person — which is exactly how the two generate-dialog
    /// references came to pass on a runner with no settings file and fail on the
    /// machine they were written on.
    pub(crate) fn of(project: &Project, root: &Path, settings: &Settings) -> Self {
        let spent = spending::so_far(project, root);
        Self {
            priced: quote::generation(project, root),
            spent_cents: spent.total(),
            superseded: spent.superseded,
            ceiling_cents: settings.budget_cents,
        }
    }

    /// Why nothing can be priced, when a brief asks for something there is no
    /// rate for — the run refuses the same project in the same words.
    pub(crate) fn unquotable(&self) -> Option<String> {
        self.priced.as_ref().err().map(ToString::to_string)
    }

    /// The items of one asset kind, in document order: what the run would do
    /// with each, charged or not. Those already made are left out, as in every
    /// surface (#1436); [`made_line`](Self::made_line) counts them.
    pub(crate) fn of_kind<'a>(&'a self, project: &'a Project, kind: AssetKind) -> Vec<&'a Item> {
        self.items()
            .filter(|item| {
                project
                    .assets
                    .iter()
                    .any(|asset| asset.id.as_str() == item.subject && asset.kind == kind)
            })
            .collect()
    }

    /// Whether nothing here would be paid for — asked of the charges, as the
    /// library asks it, so a brief priced at nothing still counts as one.
    pub(crate) fn free(&self) -> bool {
        self.priced.as_ref().map_or(true, quote::Quote::is_free)
    }

    /// The whole estimate, in US cents.
    pub(crate) fn total_cents(&self) -> u64 {
        self.priced.as_ref().map_or(0, quote::Quote::cents)
    }

    /// The total, as a person reads money.
    pub(crate) fn total(&self) -> String {
        dollars(from_cents(self.total_cents()), 2)
    }

    /// Whether this run would cross the ceiling — asked of [`Budget`], the same
    /// check `scorsese generate` makes, so the window cannot say yes to a run
    /// the library would refuse.
    pub(crate) fn over_budget(&self) -> bool {
        Budget::new(self.ceiling_cents.unwrap_or(u64::MAX), self.spent_cents)
            .check(self.total_cents())
            .is_err()
    }

    /// The line about the ceiling, or `None` when nobody set one.
    pub(crate) fn ceiling(&self) -> Option<String> {
        let ceiling = self.ceiling_cents?;
        Some(format!(
            "{} spent of a {} ceiling",
            dollars(from_cents(self.spent_cents), 2),
            dollars(from_cents(ceiling), 2)
        ))
    }

    /// How many items are already made or on their way, and left unlisted.
    pub(crate) fn made(&self) -> usize {
        self.priced.as_ref().map_or(0, quote::Quote::made)
    }

    /// The line that stands in for the made items, or `None` when there are none.
    pub(crate) fn made_line(&self) -> Option<String> {
        let made = self.made();
        (made > 0).then(|| format!("{made} more already made — nothing to pay."))
    }

    /// Every item to show, or none when the project cannot be priced.
    fn items(&self) -> impl Iterator<Item = &Item> {
        self.priced.iter().flat_map(quote::Quote::shown)
    }
}

#[cfg(test)]
mod tests;
