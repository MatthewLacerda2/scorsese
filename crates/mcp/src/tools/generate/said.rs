//! What a `generate` run reads as: one line per brief, then the one total that
//! spans every provider, then what the whole project has spent, retries
//! included.

use std::path::Path;

use scorsese_core::Project;
use scorsese_providers::passes::Done;
use scorsese_providers::prices::{dollars, from_cents};
use scorsese_providers::spending;

/// The run, in lines.
pub(super) fn run(done: &Done, project: &Project, dir: &Path) -> String {
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
    for (id, outcome) in &done.tracks {
        lines.push(format!("{id}: {}", outcome.says()));
    }
    lines.push(format!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(from_cents(done.spent_cents()), 2)
    ));
    lines.push(spending::so_far(project, dir).says());
    lines.join("\n")
}
