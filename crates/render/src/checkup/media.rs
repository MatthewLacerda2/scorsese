//! Turning pool health into a checkup's findings.
//!
//! [`scorsese_core::asset_status`] answers *what state is this file in?*; this
//! module answers the question a checkup is actually asked — *will this
//! render?*
//! The two are not the same, and the difference is the timeline. A missing
//! file nothing references cannot stop a render, and a prompt that has not
//! been generated yet is the normal state of a project before GO, not damage.
//!
//! So severity is sorted by health **and** by whether any clip uses the asset,
//! never by asset kind.

use std::fmt;

use scorsese_core::{AssetHealth, AssetId, AssetStatus};

/// How much a finding matters, and therefore what it does to the exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The render cannot succeed. `scorsese check` exits non-zero.
    Problem,
    /// The render succeeds, but probably not the way anyone meant. Exit code
    /// stays zero — a signal that fails a build is a gate.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Problem => "problem",
            Self::Warning => "warning",
        })
    }
}

/// One thing `check` has to say about one asset's media.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The asset the finding is about.
    pub asset: AssetId,
    /// Whether this fails the check or merely colours it.
    pub severity: Severity,
    /// What is wrong, and what using it means — ready to print.
    pub detail: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "asset `{}`: {}", self.asset, self.detail)
    }
}

/// Every asset with something to report, in assets-table order.
///
/// Assets whose health says nothing about renderability — healthy files,
/// inline text, prompts still awaiting generation — produce no finding at all.
pub fn findings(rows: &[AssetStatus]) -> Vec<Finding> {
    rows.iter()
        .filter_map(|row| {
            let severity = severity(row)?;
            Some(Finding {
                asset: row.id.clone(),
                severity,
                detail: format!("{} ({})", describe(&row.health), uses(row.clip_count)),
            })
        })
        .collect()
}

/// How serious this row is.
///
/// `None` means silence. Two things demote a fault by one step, off the bottom
/// of the ladder into silence:
///
/// - **Nothing uses it.** Nothing an unreferenced asset does can break a
///   render — a missing file no clip shows is worth mentioning, and `gc` is
///   the answer to it, but it is not a reason to refuse.
/// - **It is a prompt.** A generated file that has gone renders as a slug card
///   and a note rather than stopping the render, and the media can be made
///   again. That is a warning about a preview, not an unrenderable project.
fn severity(row: &AssetStatus) -> Option<Severity> {
    let worst = match &row.health {
        // Not faults: healthy media, text that lives in the document, and a
        // sketch clip whose media does not exist yet by design.
        AssetHealth::Ok | AssetHealth::Inline | AssetHealth::Awaiting(_) => return None,
        // No file to decode. The render dies partway through — the exact gap
        // this command exists to close.
        AssetHealth::Missing | AssetHealth::Unreadable(_) => Severity::Problem,
        // Both render. One renders different media than was imported; the
        // other renders fine and merely leaves the pool under-described.
        AssetHealth::HashMismatch { .. } | AssetHealth::Unprobed => Severity::Warning,
    };
    if row.clip_count > 0 && !row.kind.is_generated() {
        Some(worst)
    } else {
        demote(worst)
    }
}

/// One step down the severity ladder, off the bottom of it into silence.
fn demote(severity: Severity) -> Option<Severity> {
    match severity {
        Severity::Problem => Some(Severity::Warning),
        Severity::Warning => None,
    }
}

fn describe(health: &AssetHealth) -> String {
    match health {
        AssetHealth::Missing => "its file is not there".to_owned(),
        AssetHealth::Unreadable(why) => format!("its file could not be read: {why}"),
        AssetHealth::HashMismatch { recorded, found } => format!(
            "its file changed since import (recorded {}, found {})",
            short(recorded),
            short(found)
        ),
        AssetHealth::Unprobed => {
            "nothing has probed it, so its duration and size are unknown".to_owned()
        }
        // Unreachable through `findings`, which drops these before describing
        // them. Spelled out anyway so adding a health cannot silently print
        // an empty line.
        AssetHealth::Ok => "ok".to_owned(),
        AssetHealth::Inline => "content lives in project.json".to_owned(),
        AssetHealth::Awaiting(state) => format!("awaiting generation ({state:?})"),
    }
}

/// What the clip count means for whoever is repairing this.
fn uses(clip_count: usize) -> String {
    match clip_count {
        0 => "no clip uses it — `scorsese assets gc` clears it".to_owned(),
        1 => "1 clip uses it".to_owned(),
        many => format!("{many} clips use it"),
    }
}

/// Enough hash to tell two apart without wrapping the line.
fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}
