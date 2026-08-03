//! Filling in the metadata nobody has read yet.
//!
//! This tool exists here, and not only on the command line, because an agent
//! writing `project.json` directly is precisely the path that creates assets
//! nothing has probed. The fix has to be reachable from where the problem is
//! made — otherwise the answer to "why does this feature not work on my
//! project?" is a command the client cannot run.

use scorsese_core::{ProbeOutcome, Probed, Reprobe, probe_assets};
use scorsese_render::Ffprobe;
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Read what the media pool is actually made of.
pub(crate) struct Probe;

impl Tool for Probe {
    fn name(&self) -> &'static str {
        "project_probe"
    }

    fn description(&self) -> &'static str {
        "Ask ffprobe about every asset that has a file and no recorded \
         metadata, and write down what it says. That is how long the source is, \
         its width and height, its frame rate, and whether it carries sound. \
         Import does this \
         for what it brings in, so this is for the assets that reached \
         project.json another way — which is every asset you added by writing \
         the document. Call it after adding assets by hand: features that need \
         a source's own length, a clip's trim ceiling among them, cannot work \
         on an asset nobody has looked at. Safe to re-run; an already-probed \
         asset is left alone unless `all` says otherwise."
    }

    fn costs(&self) -> Costs {
        Costs::Probe
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "all": {
                    "type": "boolean",
                    "description": "Read every file again, replacing metadata that is \
                                    already recorded. For when what is written down is \
                                    wrong. The default probes only the assets nobody \
                                    has looked at, which is what makes this cheap to \
                                    call after every edit."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let reprobe = if arguments.get("all").and_then(Value::as_bool) == Some(true) {
            Reprobe::All
        } else {
            Reprobe::Skip
        };

        // Discovered per call rather than held, for the same reason `render`
        // does it: a server that found ffprobe at startup would keep insisting
        // it was there after someone uninstalled it.
        let probe = Ffprobe::discover().map_err(|error| format!("{error}"))?;
        let report = probe_assets(&mut project, &dir, &probe, reprobe);
        if report.is_empty() {
            return Ok("no assets with a file to probe".into());
        }

        let recorded = count(&report, &ProbeOutcome::Recorded);
        if recorded > 0 {
            project
                .save(&dir)
                .map_err(|error| format!("saving the project: {error}"))?;
        }
        Ok(said(&report, recorded).into())
    }
}

/// What happened, asset by asset where it matters and as a tally where it does
/// not.
///
/// Assets that were already known are counted and not named: a client reading
/// this wants the ones that changed and the ones in trouble, and a list of
/// everything that was already fine buries both.
fn said(report: &[Probed], recorded: usize) -> String {
    let known = count(report, &ProbeOutcome::AlreadyKnown);
    let mut lines = vec![format!(
        "{recorded} probed, {known} already known, {} assets with a file",
        report.len()
    )];
    for row in report {
        match &row.outcome {
            ProbeOutcome::Missing => lines.push(format!("{}: its file is not there", row.id)),
            ProbeOutcome::Failed(why) => lines.push(format!("{}: could not probe — {why}", row.id)),
            ProbeOutcome::Recorded | ProbeOutcome::AlreadyKnown => {}
        }
    }
    lines.join("\n")
}

fn count(report: &[Probed], outcome: &ProbeOutcome) -> usize {
    report.iter().filter(|row| &row.outcome == outcome).count()
}
