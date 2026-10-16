//! What a `scorsese generate` prints: one line per brief, then the one total
//! that spans every provider — a per-pass total would invite two of them being
//! read as one.

use std::path::Path;

use scorsese_core::AssetId;
use scorsese_core::placing::Shortened;
use scorsese_providers::image;
use scorsese_providers::passes::Done;
use scorsese_providers::prices::{dollars, from_cents};
use scorsese_providers::video::Run;

/// Every brief's line, then the total.
pub(super) fn run(done: &Done, project_dir: &Path) {
    if done.is_empty() {
        println!("No generated assets in this project.");
        return;
    }
    shots(&done.shots);
    stills(&done.stills);
    for (id, outcome) in &done.lines {
        println!("{id:<24} {}", outcome.says_in(project_dir));
    }
    for (id, outcome) in &done.tracks {
        println!("{id:<24} {}", outcome.says());
    }
    println!();
    println!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(from_cents(done.spent_cents()), 2)
    );
}

/// One line per shot, and how to pick up what is still going.
pub(super) fn shots(run: &Run) {
    for (id, outcome) in &run.outcomes {
        println!("{id:<24} {}", outcome.says());
    }
    let in_flight = run.in_flight();
    if in_flight > 0 {
        println!(
            "{in_flight} still generating. The tickets are in project.json, so nothing is \
             lost — run `scorsese generate --collect` later to pick them up."
        );
    }
}

/// One line per still.
pub(super) fn stills(outcomes: &[(AssetId, image::Outcome)]) {
    for (id, outcome) in outcomes {
        println!("{id:<24} {}", outcome.says());
    }
}

/// One line per clip shortened, after the run's own report.
pub(super) fn shortened(shortened: &[Shortened]) {
    for clip in shortened {
        println!("{}", clip.says());
    }
}
