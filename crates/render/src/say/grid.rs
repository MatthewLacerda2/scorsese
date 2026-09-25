//! Which instrument is quiet in which section: one row per track, one column
//! per section, and the mean in each cell.
//!
//! The two tables above it each answer half of that. A section row is the
//! whole mix over a stretch of time, so it says *that* the trio is a decibel
//! down; a layer row is one instrument over the whole piece, so it says the
//! brass averages −25 dB over a song it plays half of. Which instrument is
//! quiet *in the trio* is the question a mix note is actually about, and
//! without this it is an inference from which tracks the arrangement plays
//! where.
//!
//! **A grid, printed always, and only the mean in it.** The choice is between
//! this, a grid on request, and printing only the tracks that move by more
//! than some threshold between sections:
//!
//! - *On request* would mean asking again, and the question comes up while
//!   reading the report that raised it — by when the bake is cached and
//!   nothing is measured any more. The grid has to arrive with the numbers it
//!   explains or it arrives at the cost of a re-render.
//! - *Only what changes* is a threshold, which is a taste: the trio that
//!   prompted this was one decibel quieter, and any threshold loose enough to
//!   keep a report short hides a finding that size.
//! - *Always, one number a cell* keeps the table as tall as the mix has
//!   tracks — the same height as the layer table — and spends the width on
//!   sections instead of on columns the rows beside it already carry. The
//!   band shares and the width per track per section stay unsaid; the level
//!   is what the question is about, and a cell with six numbers in it is a
//!   report nobody scans.
//!
//! Like everything in a report, a signal: nothing here can fail a bake.

use scorsese_zimmer::level::Layer;

/// What the first column's header says — and so how to read every cell.
const TITLE: &str = "mean by section";

/// What a column past the arrangement's end is headed: the ring-out, which
/// the song's `tail` field names.
const TAIL: &str = "tail";

/// What a cell says when that track made no sound over that section.
const SILENT: &str = "silent";

/// The quietest mean a cell reports as a number, in dBFS: the floor of the
/// 16-bit file a bake is written as, so anything under it does not survive
/// into the audio at all.
///
/// What lands under it in practice is the last whisper of a note that ended
/// in the section before — a bass reading `-151.1` in a strain it sits out.
/// That is a true measurement and a misleading cell: the question the grid
/// answers is whether an instrument is *playing* there, and a number reads as
/// "yes, softly". So it says `silent`, which is what a listener hears.
const INAUDIBLE_DBFS: f64 = -96.0;

/// The grid: a header naming the sections, then one row per track, or nothing
/// when there is no grid to draw — fewer than two tracks, or fewer than two
/// sections, the rules the tables either side of it already follow.
///
/// Returned as rows without indentation, as the other tables are, so the CLI
/// and MCP each decide how far in it sits.
pub fn grid(layers: &[Layer]) -> Vec<String> {
    let Some(first) = layers.first() else {
        return Vec::new();
    };
    let columns = first.sections.len();
    // Every layer is cut at the same boundaries, so the counts agree; a grid
    // whose rows did not would put a number under the wrong section, and
    // saying nothing is the better failure.
    if layers.len() < 2 || columns < 2 || layers.iter().any(|layer| layer.sections.len() != columns)
    {
        return Vec::new();
    }
    let headers: Vec<&str> = first
        .sections
        .iter()
        .map(|span| span.label.as_deref().unwrap_or(TAIL))
        .collect();
    let widths: Vec<usize> = headers
        .iter()
        .map(|header| header.len().max(SILENT.len()))
        .collect();
    let name = layers
        .iter()
        .map(|layer| layer.name.len())
        .chain([TITLE.len()])
        .max()
        .unwrap_or(0);

    let mut rows = vec![line(TITLE, name, headers.iter().copied(), &widths)];
    rows.extend(layers.iter().map(|layer| {
        let cells = layer.sections.iter().map(|span| {
            span.loudness
                .mean_dbfs
                .filter(|mean| *mean >= INAUDIBLE_DBFS)
                .map_or_else(|| SILENT.to_owned(), |mean| format!("{mean:.1}"))
        });
        line(&layer.name, name, cells, &widths)
    }));
    rows
}

/// One row: a name padded to the first column, then each cell right-aligned
/// under its header so a column of decibels can be read downwards.
fn line<S: AsRef<str>>(
    name: &str,
    name_width: usize,
    cells: impl Iterator<Item = S>,
    widths: &[usize],
) -> String {
    let mut said = format!("{name:<name_width$}");
    for (cell, width) in cells.zip(widths) {
        said.push_str(&format!("  {:>width$}", cell.as_ref()));
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_zimmer::level::{Loudness, Span};

    fn span(label: Option<&str>, mean: Option<f64>) -> Span {
        Span {
            label: label.map(str::to_owned),
            from_seconds: 0.0,
            to_seconds: 1.0,
            loudness: Loudness {
                peak_dbfs: mean,
                true_peak_dbfs: mean,
                mean_dbfs: mean,
            },
            bands: None,
            correlation: None,
        }
    }

    fn layer(name: &str, means: [Option<f64>; 3]) -> Layer {
        let labels = [Some("strain"), Some("trio"), None];
        Layer {
            name: name.to_owned(),
            level: span(None, Some(-20.0)),
            sections: labels
                .into_iter()
                .zip(means)
                .map(|(label, mean)| span(label, mean))
                .collect(),
        }
    }

    /// The finding the grid exists for: which track is down in which section,
    /// said in a cell rather than inferred.
    #[test]
    fn each_cell_is_one_track_over_one_section() {
        let rows = grid(&[
            layer("piano", [Some(-18.0), Some(-18.2), Some(-40.0)]),
            layer("brass", [Some(-21.0), None, Some(-45.5)]),
            layer("bass", [Some(-151.1), Some(-24.8), Some(-96.0)]),
        ]);
        assert_eq!(
            rows,
            vec![
                "mean by section  strain    trio    tail",
                "piano             -18.0   -18.2   -40.0",
                "brass             -21.0  silent   -45.5",
                "bass             silent   -24.8   -96.0",
            ]
        );
    }

    /// No second track or no second section is no grid: it would repeat a row
    /// already printed above it.
    #[test]
    fn a_grid_with_nothing_to_compare_is_not_drawn() {
        let one = layer("piano", [Some(-18.0), Some(-18.0), None]);
        assert!(grid(std::slice::from_ref(&one)).is_empty());
        let mut unsectioned = one.clone();
        unsectioned.sections.clear();
        assert!(grid(&[unsectioned.clone(), unsectioned]).is_empty());
        assert!(grid(&[]).is_empty());
    }
}
