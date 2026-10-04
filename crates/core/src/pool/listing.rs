//! The pool as a listing shows it: a sequence's stills under the sequence.

use std::collections::{HashMap, HashSet};

use crate::project::Project;

use super::status::AssetStatus;

/// One top-level line of a listing, and the stills listed under it.
///
/// `stills` is empty for everything but an image sequence. For a sequence it
/// is each still once, in the order the sequence first shows it — a still the
/// sequence repeats is one file, and one row (#684).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed<'a> {
    /// The asset this line is about.
    pub row: &'a AssetStatus,
    /// The stills listed under it, which appear nowhere else in the listing.
    pub stills: Vec<&'a AssetStatus>,
}

/// Groups [`super::asset_status`]'s rows the way every listing shows them:
/// the table's order, with each still moved under the sequence that owns it
/// ([`AssetStatus::sequence`]).
///
/// One grouping for the desktop panel, `scorsese assets` and `project_assets`,
/// so the three cannot disagree about what is listed where. Every row appears
/// exactly once, at the top level or under one sequence.
pub fn listing<'a>(project: &Project, rows: &'a [AssetStatus]) -> Vec<Listed<'a>> {
    let by_id: HashMap<_, _> = rows.iter().map(|row| (&row.id, row)).collect();
    rows.iter()
        .filter(|row| row.sequence.is_none())
        .map(|row| {
            let mut seen = HashSet::new();
            let stills = project
                .asset(&row.id)
                .and_then(|asset| asset.sequence.as_ref())
                .map(|sequence| {
                    sequence
                        .stills
                        .iter()
                        .filter(|still| seen.insert(*still))
                        .filter_map(|still| by_id.get(still).copied())
                        .filter(|still| still.sequence.as_ref() == Some(&row.id))
                        .collect()
                })
                .unwrap_or_default();
            Listed { row, stills }
        })
        .collect()
}
