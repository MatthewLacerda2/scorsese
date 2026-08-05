//! Handing briefs to a provider without the window waiting for it.
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
//! The pattern is [`Probing`](crate::project::Probing)'s: a channel, a copy to
//! the worker, and a poll from the repaint loop that never blocks.

use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use scorsese_core::Project;
use scorsese_providers::credentials::{Budget, Provider, Settings, resolve};
use scorsese_providers::prices::dollars;
use scorsese_providers::video::{Outcome, VeoProvider, collect, generate_waiting};

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
    let key = match resolve(Provider::Gemini) {
        Ok(key) => key,
        Err(error) => return failure(error.to_string()),
    };
    let provider = VeoProvider::new(&key.secret);
    let settings = Settings::load().unwrap_or_default();
    let budget = Budget::from_settings(&settings, super::quote::spent(&project));

    let outcome = match pass {
        // Zero patience: the submit itself is what is wanted, and the window
        // does the waiting by sweeping.
        Pass::Submit => generate_waiting(
            &mut project,
            root,
            &provider,
            budget,
            Duration::ZERO,
            |_| {},
        )
        .map(|run| (run.outcomes, run.spent_cents)),
        Pass::Sweep => collect(&mut project, root, &provider).map(|outcomes| (outcomes, 0)),
    };

    let (outcomes, spent) = match outcome {
        Ok(both) => both,
        // Saved anyway before reporting: a ticket written just before a failure
        // is the only record that money was spent, and dropping it means paying
        // for that work twice.
        Err(error) => {
            let _ = project.save(root);
            return failure(error.to_string());
        }
    };
    if let Err(error) = project.save(root) {
        return failure(format!("saving the project: {error}"));
    }
    Report {
        in_flight: outcomes
            .iter()
            .filter(|(_, outcome)| outcome.is_in_flight())
            .count(),
        said: said(&outcomes, spent, pass),
        failed: false,
    }
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
fn said(outcomes: &[(scorsese_core::AssetId, Outcome)], spent: u64, pass: Pass) -> String {
    let generated = outcomes
        .iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
        .count();
    let refused = outcomes
        .iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Failed { .. }))
        .count();
    let expired = outcomes
        .iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Expired { .. }))
        .count();
    let waiting = outcomes
        .iter()
        .filter(|(_, outcome)| outcome.is_in_flight())
        .count();

    let mut parts = Vec::new();
    if waiting > 0 {
        parts.push(format!("{waiting} generating"));
    }
    if generated > 0 {
        parts.push(format!("{generated} arrived"));
    }
    if refused > 0 {
        parts.push(format!("{refused} refused"));
    }
    if expired > 0 {
        parts.push(format!("{expired} past the window and lost"));
    }
    if parts.is_empty() {
        return match pass {
            Pass::Submit => String::from("nothing to generate — every shot is already made"),
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
