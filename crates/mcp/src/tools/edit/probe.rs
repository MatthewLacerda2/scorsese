//! Filling in the metadata nobody has read yet, as part of a write.
//!
//! This used to be a tool of its own, `project_probe`, whose description told a
//! client to call it after adding assets by writing the document — because an
//! agent writing `project.json` directly is precisely the path that creates
//! assets nothing has probed. A tool whose whole job is "remember to call me
//! after the other one" is a step the other one should take (#783), so
//! `project_write` takes it: the way `import` probes what it brings in, a write
//! probes what it adds.

use std::path::Path;

use scorsese_core::{ProbeOutcome, Probed, Project, Reprobe, probe_assets, unprobed_assets};
use scorsese_render::Ffprobe;

/// Probe what `project` adds that nobody has measured — or, with
/// [`Reprobe::All`], every file it names — and say what came of it, one line
/// a thing worth saying. Nothing to say is no lines.
///
/// Never a refusal. A file that is not there, or that ffprobe cannot read, is
/// reported and its asset left exactly as it was written; a machine with no
/// ffprobe at all writes the document unmeasured and says so. What a probe
/// *found* can still refuse the write — a clip longer than the source it was
/// just measured to be — but that is validation's answer, given afterwards.
pub(super) fn measured(project: &mut Project, dir: &Path, reprobe: Reprobe) -> Vec<String> {
    // Asked without touching the disk, so a write that adds no footage — the
    // usual one — spawns no process and never needs ffprobe to exist.
    if reprobe == Reprobe::Skip && unprobed_assets(project).is_empty() {
        return Vec::new();
    }
    // Discovered per call rather than held, for the same reason `render`
    // does it: a server that found ffprobe at startup would keep insisting
    // it was there after someone uninstalled it.
    let probe = match Ffprobe::discover() {
        Ok(probe) => probe,
        Err(error) => return vec![format!("not probed: {error}")],
    };
    let report = probe_assets(project, dir, &probe, reprobe);
    said(&report)
}

/// What happened, asset by asset where it matters and as a tally where it does
/// not.
///
/// Assets that were already known are not mentioned: a client reading this
/// wants the ones that changed and the ones in trouble, and a list of
/// everything that was already fine buries both.
fn said(report: &[Probed]) -> Vec<String> {
    let recorded = report
        .iter()
        .filter(|row| row.outcome == ProbeOutcome::Recorded)
        .count();
    let mut lines = Vec::new();
    if recorded > 0 {
        lines.push(format!("probed {recorded} asset(s)"));
    }
    for row in report {
        match &row.outcome {
            ProbeOutcome::Missing => lines.push(format!("{}: its file is not there", row.id)),
            ProbeOutcome::Failed(why) => lines.push(format!("{}: could not probe — {why}", row.id)),
            ProbeOutcome::Recorded | ProbeOutcome::AlreadyKnown => {}
        }
    }
    lines
}
