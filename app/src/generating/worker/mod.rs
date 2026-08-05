//! Handing briefs to the providers without the window waiting for them.
//!
//! **Submit and detach; never wait.** `scorsese generate` waits five minutes
//! because a terminal can afford to. A window cannot — a frozen picture is the
//! one thing this app exists not to have — so the button hands the briefs over
//! and returns, and the window then sweeps for what has finished.
//!
//! Keeping each background operation *short* is what makes this safe rather
//! than merely responsive. The worker gets a copy of the document and saves it,
//! and so does the window; a five-minute overlap would be five minutes in which
//! an edit could be clobbered by an answer to a question asked before it. A
//! submit is a second or two, and a sweep is less.
//!
//! The pattern is [`Probing`](crate::project::probing::Probing)'s: a channel, a
//! copy to the worker, and a poll from the repaint loop that never blocks.
//!
//! # Two vendors, one ceiling, one total
//!
//! [`shots`] is Veo and [`lines`] is ElevenLabs, run in that order with the
//! ceiling threaded from the first into the second — a limit each pass checked
//! on its own would be a limit worth twice what somebody set. Each resolves its
//! own key, and only when it has work: a project of nothing but narration must
//! not be stopped for want of a Veo key, and the other way round.

mod lines;
mod shots;

use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use scorsese_core::{Asset, AssetKind, Project};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::prices::dollars;

/// What one background pass did.
pub(crate) struct Report {
    /// The sentence to put on screen.
    pub(crate) said: String,
    /// Whether anything is still in flight, which is what decides whether the
    /// window keeps sweeping.
    pub(crate) in_flight: usize,
    /// True when the pass could not run at all — no key, over the ceiling. The
    /// window stops sweeping rather than saying the same thing every ten
    /// seconds.
    pub(crate) failed: bool,
}

/// A submit or a sweep, running somewhere else.
pub(crate) struct Working {
    answer: Receiver<Report>,
}

/// Which pass to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Hand over everything that needs it, then return without waiting.
    Submit,
    /// Poll what is already in flight. **Cannot spend anything**, which is what
    /// makes it safe to run on a timer.
    Sweep,
}

impl Working {
    /// Starts a pass over a copy of `project`.
    pub(crate) fn start(pass: Pass, project: &Project, root: &Path) -> Self {
        let (sender, answer) = channel();
        let copy = project.clone();
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            let _ = sender.send(run(pass, copy, &root));
        });
        Self { answer }
    }

    /// The report, if the worker has it yet. Never blocks.
    pub(crate) fn finished(&mut self) -> Option<Report> {
        match self.answer.try_recv() {
            Ok(report) => Some(report),
            // The worker died without reporting. Said as a failure rather than
            // as nothing, because a button that silently does nothing is worse
            // than one that says it could not.
            Err(TryRecvError::Disconnected) => Some(Report {
                said: String::from("the generation worker stopped without saying why"),
                in_flight: 0,
                failed: true,
            }),
            Err(TryRecvError::Empty) => None,
        }
    }
}

/// One pass, on the worker's thread.
fn run(pass: Pass, mut project: Project, root: &Path) -> Report {
    let settings = Settings::load().unwrap_or_default();
    let budget = Budget::from_settings(&settings, super::quote::spent(&project));

    let made = match shots::pass(&mut project, root, pass, budget) {
        Ok(made) => made,
        Err(said) => return stopped(&project, root, said),
    };
    let spoken = match lines::pass(&mut project, root, pass, budget.spend(made.spent_cents)) {
        Ok(spoken) => spoken,
        Err(said) => return stopped(&project, root, said),
    };

    if let Err(error) = project.save(root) {
        return failure(format!("saving the project: {error}"));
    }
    let mut parts = shots::said(&made.outcomes);
    parts.extend(lines::said(&spoken));
    Report {
        in_flight: made
            .outcomes
            .iter()
            .filter(|(_, outcome)| outcome.is_in_flight())
            .count(),
        said: sentence(parts, made.spent_cents + lines::spent(&spoken), pass),
        failed: false,
    }
}

/// A pass that stopped partway, with the document saved first.
///
/// Saved before reporting rather than after: a ticket written just before a
/// failure is the only record that money was spent, and dropping it means
/// paying for that work twice.
fn stopped(project: &Project, root: &Path, said: String) -> Report {
    let _ = project.save(root);
    failure(said)
}

/// A pass that could not run.
fn failure(said: String) -> Report {
    Report {
        said,
        in_flight: 0,
        failed: true,
    }
}

/// What the pass reads as, in one line.
fn sentence(mut parts: Vec<String>, spent: u64, pass: Pass) -> String {
    if parts.is_empty() {
        return match pass {
            Pass::Submit => {
                String::from("nothing to generate — every shot and every line is already made")
            }
            Pass::Sweep => String::from("nothing in flight"),
        };
    }
    if spent > 0 {
        // Always with the qualifier. This is a number somebody reads before
        // deciding to spend more, and no provider tells us what was billed.
        parts.push(format!(
            "about {} spent, by our calculation",
            dollars(spent)
        ));
    }
    parts.join(" · ")
}

/// Whether this kind has anything left to do, and so whether its key is needed.
///
/// True when a brief of that kind has no file behind it yet, or has work in
/// flight. The **disk** and not the document, deliberately: an asset can say
/// `generated` and have lost its file — deleted, or never copied along with the
/// project — and that one does need making again, whatever the state field
/// claims.
fn wants(project: &Project, root: &Path, kind: AssetKind) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == kind)
        .any(|asset| asset.operation.is_some() || !present(asset, root))
}

/// Whether this asset's media is actually on disk.
fn present(asset: &Asset, root: &Path) -> bool {
    asset
        .path
        .as_ref()
        .is_some_and(|path| path.resolve(root).is_file())
}

/// One clause of a report — `3 arrived` — and nothing at all when the count is
/// zero.
fn counted(count: usize, what: &str) -> Option<String> {
    (count > 0).then(|| format!("{count} {what}"))
}
