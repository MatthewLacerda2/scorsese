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
//! # Three passes, one ceiling, one total
//!
//! [`shots`] is Veo, [`stills`] Gemini and [`lines`] ElevenLabs, run in that
//! order with the ceiling threaded from each into the next — a limit each pass
//! checked on its own would be a limit worth twice what somebody set. Each
//! resolves its own key, and only when it has work: a project of nothing but
//! narration must not be stopped for want of a Veo key, and the other way
//! round. A pass that stops part-way ends the run, and the line says what was
//! made before the reason it stopped (#1153).

mod lines;
mod shots;
mod stills;

use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use scorsese_core::placing::{self, Shortened};
use scorsese_core::{AssetId, Project, Reprobe, probe_assets};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::prices::dollars;
use scorsese_providers::spending;
use scorsese_providers::video::Run;
use scorsese_providers::{image, speech};
use scorsese_render::Ffprobe;

/// What one background pass did.
pub(crate) struct Report {
    /// The sentence to put on screen.
    pub(crate) said: String,
    /// Whether anything is still in flight, which is what decides whether the
    /// window keeps sweeping.
    pub(crate) in_flight: usize,
    /// True when the pass could not run at all — no key, over the ceiling —
    /// or stopped part-way with nothing left in flight. The window stops
    /// sweeping rather than saying the same thing every ten seconds.
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
    // A settings file that is there and unreadable refuses to spend: read as
    // the defaults, it would be a run with no ceiling at all (#1108).
    let settings = match Settings::load() {
        Ok(settings) => settings,
        Err(error) => return failure(format!("nothing was sent: {error}")),
    };
    let budget = Budget::from_settings(&settings, spending::so_far(&project, root).total());

    let mut made = Run {
        outcomes: Vec::new(),
        spent_cents: 0,
    };
    let mut drawn = Vec::new();
    let mut spoken = Vec::new();
    let stop = passes(
        &mut project,
        root,
        pass,
        budget,
        &mut made,
        &mut drawn,
        &mut spoken,
    )
    .err();
    // A pass that could not start, with nothing done before it, is the
    // reason alone. One that stopped after making something says what it
    // made first: the reason alone reads as nothing having happened (#1153).
    if let Some(said) = &stop
        && made.outcomes.is_empty()
        && drawn.is_empty()
        && spoken.is_empty()
    {
        return stopped(&project, root, said.clone());
    }
    let committed = made.spent_cents + stills::spent(&drawn);

    // Before the save, so what was measured is in the document the window
    // reloads rather than in a copy this thread is about to drop.
    let landed: Vec<AssetId> = shots::arrived(&made.outcomes)
        .chain(lines::arrived(&spoken))
        .cloned()
        .collect();
    let shortened = if !landed.is_empty() || stills::arrived(&drawn) {
        measure(&mut project, root, &landed)
    } else {
        Vec::new()
    };
    if let Err(error) = project.save(root) {
        return failure(format!("saving the project: {error}"));
    }
    let mut parts = shots::said(&made.outcomes);
    parts.extend(stills::said(&drawn));
    parts.extend(lines::said(&spoken));
    parts.extend(shortened.iter().map(Shortened::says));
    let in_flight = made.in_flight();
    let mut said = sentence(parts, committed + lines::spent(&spoken), pass);
    if let Some(stop) = &stop {
        said = format!("{said} · {stop}");
    }
    Report {
        in_flight,
        said,
        // Shots a stopped run sent are still worth sweeping for: collecting
        // spends nothing, and whatever stopped it does not stop that.
        failed: stop.is_some() && in_flight == 0,
    }
}

/// Every pass, in order, each into its own slot as it goes — so one that
/// stops leaves the earlier ones' work, and its own as far as it got, for the
/// report. A pass that stops ends the run, the ceiling threaded through.
fn passes(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
    made: &mut Run,
    drawn: &mut Vec<(AssetId, image::Outcome)>,
    spoken: &mut Vec<(AssetId, speech::Outcome)>,
) -> Result<(), String> {
    shots::pass(project, root, pass, budget, made)?;
    stills::pass(project, root, pass, budget.spend(made.spent_cents), drawn)?;
    let committed = made.spent_cents + stills::spent(drawn);
    lines::pass(project, root, pass, budget.spend(committed), spoken)
}

/// Measures whatever has just landed, for both vendors at once.
///
/// **A generation has no measured shape until something looks**, and neither
/// kind of brief can supply one. How long a line takes depends on the words;
/// whether Veo put an engine note under a shot is not in the prompt either, and
/// it routinely does. `scorsese-providers` can answer neither — it has no
/// decoder and must not grow one. The mix reads `duration_seconds` and
/// `audio_channels`, so a generation nobody probed is a line the mix skips and
/// a shot that plays silent.
///
/// **And a measured generation can be shorter than its clip**: a reading takes
/// as long as its words do, whatever its sketch was laid out at. The clips of
/// what `landed` that now outlast it are shortened to it and handed back to be
/// said ([`placing::fit_to_sources`], #825) — left alone, the window's reload
/// would refuse the document (`ClipOutlastsSource`).
///
/// A failure here is not reported and not an error. The media is on disk and
/// has been paid for; a machine with no ffprobe should not be told its run
/// failed, and both `scorsese probe` and this window's own open-time pass are
/// still there to close the gap.
fn measure(project: &mut Project, root: &Path, landed: &[AssetId]) -> Vec<Shortened> {
    let Ok(probe) = Ffprobe::discover() else {
        return Vec::new();
    };
    probe_assets(project, root, &probe, Reprobe::Skip);
    placing::fit_to_sources(project, landed)
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

/// One clause of a report — `3 arrived` — and nothing at all when the count is
/// zero.
fn counted(count: usize, what: &str) -> Option<String> {
    (count > 0).then(|| format!("{count} {what}"))
}
