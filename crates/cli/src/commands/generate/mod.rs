//! `scorsese generate`
//!
//! The command that spends money, and the only one in the binary that does.
//! Everything it prints is written on that assumption: what each brief did,
//! what the run is estimated to have cost, and what is still in flight.
//!
//! **Three passes, one ceiling, one total.** Shots, stills and narration are
//! separate passes because they are separate models and vendors, but the budget
//! is threaded from each into the next and the totals are added — a ceiling that each
//! pass checked independently would be a ceiling worth twice what somebody set.
//!
//! **A key is asked for only by a pass that has work.** Resolving both up front
//! would mean a project of nothing but narration could not be generated without
//! a Veo key, and the other way round. Which is what each provider's `pending`
//! decides — from the current brief, never the asset's recorded state.
//!
//! **Spending is opt-in.** Nothing is submitted until the quote has been shown
//! and somebody has said go ahead — see [`confirm`](super::confirm). The ceiling is checked
//! further down still, inside the pass, where no flag reaches it.

mod measure;
mod report;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use scorsese_core::Project;
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::Order;
use scorsese_providers::passes::{self, Halt, Keyed};
use scorsese_providers::prices::{dollars, from_cents};
use scorsese_providers::{image, music, speech, video};

use super::confirm;

/// What one `scorsese generate` was asked for, beyond the project.
pub(crate) struct Asked {
    /// How long to wait for the shots before detaching.
    pub(crate) patience: Duration,
    /// Quote and stop.
    pub(crate) dry_run: bool,
    /// Go ahead without asking.
    pub(crate) yes: bool,
    /// Order the stills as a half-price batch (#894) rather than now.
    pub(crate) batch: bool,
}

impl Asked {
    /// How the stills are ordered.
    fn order(&self) -> Order {
        if self.batch { Order::Batch } else { Order::Now }
    }
}

/// Realises every sketched brief, waiting up to `patience` for the shots — or,
/// asked for a batch, orders the stills and nothing else.
pub(crate) fn run(project_dir: &Path, asked: &Asked) -> Result<()> {
    let mut project = super::open(project_dir)?;
    let order = asked.order();

    if asked.dry_run {
        return quote(&project, project_dir, order);
    }

    if pending(&project, project_dir) && !permitted(&project, project_dir, asked.yes, order)? {
        println!("Nothing was sent.");
        return Ok(());
    }

    let budget = Budget::for_project(&project, project_dir).context("nothing was sent")?;
    let passes_asked = passes::Asked {
        budget,
        patience: asked.patience,
        order,
    };

    // The document is saved whatever happens, and that is not tidiness: a
    // ticket written before a failure is the only record that money was spent,
    // and losing it means paying again for work already in flight.
    let (done, outcome) = passes::run(&mut project, project_dir, &passes_asked, &Keyed);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    // What landed is measured even when the run stopped after it: a line
    // that came back shorter than its clip leaves a project nothing can open
    // until its clip is fitted (#1114).
    let measured = if done.unmeasured() {
        measure::measure(&mut project, project_dir, &done)
    } else {
        Ok(Vec::new())
    };
    // A run that stopped after spending says what it spent before it says
    // why it stopped: the error alone reads as nothing having happened (#1057).
    if !done.is_empty() || outcome.is_ok() {
        report::run(&done, project_dir);
    }
    if let Ok(shortened) = &measured {
        report::shortened(shortened);
    }
    outcome.map_err(halted)?;
    measured.map(drop)
}

/// Why a run ended early, as the error the terminal prints — a missing key
/// or a vendor's failure keeps its own chain of causes.
fn halted(halt: Halt) -> anyhow::Error {
    match halt {
        Halt::Key { error, .. } => error.into(),
        Halt::Stopped(says) => anyhow::anyhow!(says),
        Halt::Shots(error) => error.into(),
        Halt::Stills(error) => error.into(),
    }
}

/// Whether this run may go ahead: the quote, and then the question.
///
/// The quote is printed **only when somebody is going to be asked**, because it
/// is there to be decided on. A run that already carries its answer prints what
/// it did, not what it was about to do.
fn permitted(project: &Project, root: &Path, yes: bool, order: Order) -> Result<bool> {
    // A batch with a shot or a line in it is refused before anything is
    // asked, however the run was told to go ahead.
    if order == Order::Batch {
        scorsese_providers::quote::batch(project, root)?;
    }
    match confirm::verdict(yes, confirm::interactive()) {
        confirm::Verdict::Ahead => Ok(true),
        confirm::Verdict::NobodyThere => Err(anyhow::anyhow!(confirm::nobody_there("generate"))),
        confirm::Verdict::Ask => {
            quote(project, root, order)?;
            confirm::asked("Generate this?")
        }
    }
}

/// Whether either pass has anything to do at all.
///
/// A run with nothing to submit is never asked about and never refused for
/// having no terminal: there is no decision to take when the answer costs
/// nothing either way. It is what keeps `scorsese generate` over a project that
/// is already generated working from a script, exactly as it did before.
///
/// Asked of each provider's [`plan`](video::plan), which consults the
/// **current brief's** output file — never the asset's recorded `path`. The
/// recorded path is the previous generation's file: after an edit it still
/// exists and still resolves, and consulting it is how a stale shot used to be
/// skipped as *nothing to do*.
fn pending(project: &Project, root: &Path) -> bool {
    video::pending(project, root)
        || image::pending(project, root)
        || speech::pending(project, root)
        || music::pending(project, root)
}

/// Picks up whatever finished while nobody was watching. Submits nothing.
pub(crate) fn sweep(project_dir: &Path) -> Result<()> {
    let mut project = super::open(project_dir)?;

    let (done, outcome) = passes::collect(&mut project, project_dir, &Keyed);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    outcome.map_err(halted)?;
    if done.is_empty() {
        println!("Nothing is in flight.");
        return Ok(());
    }
    // A shot collected here is a shot nothing has looked at, exactly as if the
    // run that submitted it had waited for it — and so is a still from its
    // batch.
    let shortened = if done.unmeasured() {
        measure::measure(&mut project, project_dir, &done)?
    } else {
        Vec::new()
    };
    // A sweep spends nothing by construction, so its total is zero and saying
    // so is honest rather than a gap.
    report::shots(&done.shots);
    report::stills(&done.stills);
    report::shortened(&shortened);
    println!();
    println!(
        "About {} spent on this run — nothing, because a sweep only collects.",
        dollars(from_cents(0), 2)
    );
    Ok(())
}

/// What a run would cost, without a key and without spending anything.
///
/// What **this** run would spend, not what the project's briefs would cost
/// from scratch: a brief whose output already sits in `generated/` is listed
/// at nothing, because the run will find its file and send nothing. The
/// arithmetic is [`scorsese_providers::quote::generation`]'s — the same quote
/// the MCP tool binds its token to, so the terminal and a tool call cannot
/// price one project differently.
///
/// A quote for now offers the batch beside it when its stills are worth it
/// ([`quote::offer`](scorsese_providers::quote::offer)): the choice trades the
/// person's time, so it is put in front of them rather than made.
fn quote(project: &Project, root: &Path, order: Order) -> Result<()> {
    let quote = match order {
        Order::Now => scorsese_providers::quote::generation(project, root)?,
        Order::Batch => scorsese_providers::quote::batch(project, root)?,
    };
    for item in &quote.items {
        println!("{:<24} {}", item.subject, item.says);
    }
    println!();
    println!(
        "About {} for the whole run — calculated from the published rates, never a bill. \
         See `scorsese guide prices`.",
        dollars(from_cents(quote.cents()), 2)
    );
    if order == Order::Now
        && let Some(offer) = scorsese_providers::quote::offer(project, root)?
    {
        println!(
            "Stills: {} now · {} with --batch, ready within 24 hours (half price).",
            dollars(from_cents(offer.now_cents), 2),
            dollars(from_cents(offer.batch_cents), 2)
        );
    }
    Ok(())
}
