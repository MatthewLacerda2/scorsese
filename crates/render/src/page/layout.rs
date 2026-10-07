//! Layout notes: what a page's boxes get wrong, asked of the browser (#813).
//!
//! The page is a DOM, so the browser already knows where every line of text
//! is. After some of the frames a capture draws, `layout.js` is evaluated and
//! answers with findings — text off the frame, inside the safe margin, out of
//! the box painted behind it, or over other text — and this turns them into
//! the capture's warnings. One evaluation per sampled frame, and no pixels.
//!
//! **Only what is held is said.** A finding counts when the same one, to the
//! pixel, comes back from two samples running: an entrance sliding in from off
//! the frame changes its numbers between samples and says nothing, while a
//! caption laid out wrong sits still and is heard. A finding is said once, at
//! the first sample it held from, and a render never fails on one.

use std::collections::BTreeSet;

use serde::Deserialize;

use super::request::Request;

/// How many frames a second are measured. A layout mistake worth hearing about
/// is on screen for at least a second, the time it takes to be read.
const PER_SECOND: f64 = 4.0;

/// The safe margin, as a fraction of each side of the frame: `docs/pages.md`'s
/// 5%, which is 96 CSS pixels at the sides of a landscape page.
const MARGIN: f64 = 0.05;

/// How many layout notes one capture says before it only counts the rest: a
/// page that is wrong everywhere needs the first few, not a wall of them.
const MOST: usize = 8;

/// The expression measuring the frame just drawn.
pub(crate) fn expression() -> String {
    format!("({})({MARGIN})", include_str!("layout.js").trim_end())
}

/// Whether capture frame `k` is measured: the first, the last, and the first
/// of each quarter second between them.
pub(crate) fn sampled(request: &Request, k: u64) -> bool {
    let quarter = |k: u64| (request.millis_at(k) * PER_SECOND / 1000.0).floor();
    k == 0 || k + 1 == request.frames() || quarter(k) != quarter(k - 1)
}

/// One thing `layout.js` found on one frame.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Finding {
    /// The message and the geometry it was found at.
    pub(crate) key: String,
    /// What the author reads.
    pub(crate) message: String,
}

/// The findings of every sample so far, kept as notes once they hold.
#[derive(Debug, Default)]
pub(crate) struct Layout {
    previous: Option<(f64, Vec<Finding>)>,
    said: BTreeSet<String>,
    notes: Vec<String>,
    unsaid: usize,
}

impl Layout {
    /// What the sample at `seconds` of page clock found.
    pub(crate) fn heard(&mut self, seconds: f64, findings: Vec<Finding>) {
        if let Some((since, before)) = &self.previous {
            for finding in findings.iter().filter(|f| before.contains(f)) {
                if !self.said.insert(finding.message.clone()) {
                    continue;
                }
                if self.notes.len() < MOST {
                    let note =
                        format!("{} (from {} into the page)", finding.message, clock(*since));
                    self.notes.push(note);
                } else {
                    self.unsaid += 1;
                }
            }
        }
        self.previous = Some((seconds, findings));
    }

    /// The notes, in the order they were first held.
    pub(crate) fn notes(mut self) -> Vec<String> {
        if self.unsaid > 0 {
            self.notes.push(format!(
                "and {} more layout {} like these",
                self.unsaid,
                if self.unsaid == 1 {
                    "problem"
                } else {
                    "problems"
                }
            ));
        }
        self.notes
    }
}

/// Seconds of page clock as a timestamp a `still` can be asked for.
fn clock(seconds: f64) -> String {
    let rounded = format!("{seconds:.2}");
    format!("{}s", rounded.trim_end_matches('0').trim_end_matches('.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_compositor::Resolution;
    use scorsese_core::Fps;

    fn found(message: &str, at: &str) -> Finding {
        Finding {
            key: format!("{message} @{at}"),
            message: message.into(),
        }
    }

    #[test]
    fn the_first_last_and_each_quarter_second_are_measured() {
        let request = Request {
            page: "pages/a.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps: Fps::THIRTY,
            // Not a whole quarter, so the last frame is measured for being last.
            duration: 0.9,
            clips: Default::default(),
        };
        let measured: Vec<u64> = (0..request.frames())
            .filter(|&k| sampled(&request, k))
            .collect();
        assert_eq!(measured, [0, 8, 15, 23, 27]);
    }

    #[test]
    fn a_finding_is_said_once_it_holds_still_and_never_twice() {
        let mut layout = Layout::default();
        layout.heard(0.0, vec![found("off", "1")]);
        layout.heard(0.25, vec![found("off", "2"), found("clash", "9")]);
        layout.heard(0.5, vec![found("off", "2"), found("clash", "9")]);
        layout.heard(0.75, vec![found("off", "2")]);
        layout.heard(1.0, vec![found("clash", "9")]);
        layout.heard(1.25, vec![found("clash", "9")]);
        assert_eq!(
            layout.notes(),
            [
                "off (from 0.25s into the page)",
                "clash (from 0.25s into the page)"
            ],
            "the slide-in at 0 is not held; each held finding is said at its start"
        );
    }

    #[test]
    fn past_a_handful_the_rest_are_counted() {
        let mut layout = Layout::default();
        let many: Vec<Finding> = (0..10).map(|n| found(&format!("n{n}"), "0")).collect();
        layout.heard(0.0, many.clone());
        layout.heard(1.5, many);
        let notes = layout.notes();
        assert_eq!(notes.len(), MOST + 1);
        assert_eq!(notes[MOST], "and 2 more layout problems like these");
    }

    #[test]
    fn the_measuring_script_is_called_with_the_margin() {
        let expression = expression();
        assert!(expression.starts_with("(// What a page's layout"));
        assert!(expression.ends_with(")(0.05)"), "{expression}");
    }
}
