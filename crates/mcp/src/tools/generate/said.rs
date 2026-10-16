//! What a `generate` run reads as: one line per brief, then the one total that
//! spans every provider.

use std::path::Path;

use scorsese_providers::passes::Done;
use scorsese_providers::prices::dollars;

/// The run, in lines.
pub(super) fn run(done: &Done, dir: &Path) -> String {
    if done.is_empty() {
        return String::from(
            "Nothing to generate: every prompted brief in this project is already generated \
             (or there are none), and nothing is in flight.",
        );
    }
    let mut lines = Vec::new();
    for (id, outcome) in &done.shots.outcomes {
        lines.push(format!("{id}: {}", outcome.says()));
    }
    let in_flight = done.shots.in_flight();
    if in_flight > 0 {
        lines.push(format!(
            "{in_flight} still generating. The tickets are in project.json, so nothing is \
             lost — call again with collect: true to pick them up."
        ));
    }
    for (id, outcome) in &done.stills {
        lines.push(format!("{id}: {}", outcome.says()));
    }
    for (id, outcome) in &done.lines {
        lines.push(format!("{id}: {}", outcome.says_in(dir)));
    }
    lines.push(format!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(done.spent_cents())
    ));
    lines.join("\n")
}
