//! The question asked before anything is submitted.
//!
//! `--dry-run` made seeing a price **opt-in**: a run that spent money was the
//! one you got by typing nothing extra. This turns that round — the quote is
//! printed and the run waits to be told to go ahead, and `--yes` is how a
//! caller with nobody at the keyboard says so up front. The window has asked
//! since #220; the terminal spending more readily than the window is the wrong
//! way round.
//!
//! **The ceiling is not part of this decision, and must never become part of
//! it.** [`Budget::check`](scorsese_providers::credentials::Budget::check) sits
//! below the flag, inside the pass that submits, so a run permitted here can
//! still be refused there — which is what makes a limit `--yes` cannot wave
//! through. Moving the check up beside the flag would undo that.

use std::io::{BufRead, IsTerminal, Write};

use anyhow::Result;

/// What the flag and the terminal together say about asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Go ahead without asking: somebody already said yes.
    Ahead,
    /// Somebody is there — show them the quote and ask.
    Ask,
    /// Nothing is there to answer, and nobody said yes in advance.
    NobodyThere,
}

/// What a run with nobody at the other end is refused with.
///
/// It names the flag, because the caller reading this is a script or an agent
/// and the next thing it needs is the one word that makes the run work.
pub(super) const NOBODY_THERE: &str = "nothing is there to answer: stdin is not a terminal, and \
     `scorsese generate` asks before it spends. Pass --yes to run unattended, \
     or --dry-run to see what it would cost and send nothing.";

/// Whether to ask, go ahead, or refuse.
///
/// Pure on purpose: the terminal is read once, by [`interactive`], and every
/// combination that decides money is testable without one.
pub(super) fn verdict(yes: bool, interactive: bool) -> Verdict {
    match (yes, interactive) {
        (true, _) => Verdict::Ahead,
        (false, true) => Verdict::Ask,
        (false, false) => Verdict::NobodyThere,
    }
}

/// Whether a person is at the other end of stdin, rather than a pipe.
///
/// **stdin and not stdout**, because stdin is what an answer would arrive on.
/// A run whose output is piped to a file still has somebody typing at it.
pub(super) fn interactive() -> bool {
    std::io::stdin().is_terminal()
}

/// Asks, and waits for the answer.
pub(super) fn asked() -> Result<bool> {
    let stdin = std::io::stdin();
    answered(&mut stdin.lock(), &mut std::io::stdout())
}

/// The question, and what counts as a yes.
///
/// Anything that is not a plain yes is a no, and end-of-input is a no: the
/// default has to be the answer that spends nothing, because it is the one an
/// unsure person gets by pressing return.
fn answered(input: &mut impl BufRead, output: &mut impl Write) -> Result<bool> {
    write!(output, "\nGenerate this? Nothing has been sent yet. [y/N] ")?;
    output.flush()?;

    let mut answer = String::new();
    input.read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

#[cfg(test)]
mod tests {
    use super::{Verdict, answered, verdict};

    #[test]
    fn the_flag_answers_for_a_person_who_is_not_there() {
        assert_eq!(verdict(true, false), Verdict::Ahead);
        assert_eq!(verdict(true, true), Verdict::Ahead);
    }

    #[test]
    fn a_terminal_gets_asked_and_a_pipe_gets_refused() {
        assert_eq!(verdict(false, true), Verdict::Ask);
        assert_eq!(verdict(false, false), Verdict::NobodyThere);
    }

    #[test]
    fn the_refusal_names_the_flag_that_makes_the_run_work() {
        assert!(super::NOBODY_THERE.contains("--yes"));
    }

    /// What the answer is read as, with the question's own output thrown away.
    fn given(answer: &str) -> bool {
        let mut said = Vec::new();
        answered(&mut answer.as_bytes(), &mut said).expect("read the answer")
    }

    #[test]
    fn yes_is_the_only_yes() {
        assert!(given("y\n"));
        assert!(given("Y\n"));
        assert!(given("yes\n"));
        assert!(given("  YES  \n"));
    }

    #[test]
    fn everything_else_spends_nothing() {
        assert!(!given("\n"));
        assert!(!given(""));
        assert!(!given("n\n"));
        assert!(!given("no\n"));
        assert!(!given("yeah\n"));
        assert!(!given("ok\n"));
    }

    #[test]
    fn the_question_is_printed_before_the_answer_is_read() {
        let mut said = Vec::new();
        answered(&mut "n\n".as_bytes(), &mut said).expect("read the answer");
        let question = String::from_utf8(said).expect("the question is text");
        assert!(question.contains("[y/N]"), "asked: {question}");
    }
}
