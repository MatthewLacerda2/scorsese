//! One row per layer of a mix: which part of it is taking up the room.
//!
//! The table beside this one splits a piece by **time** and answers *when* it
//! is muddy. This one splits it by **part** and answers *which of the things
//! playing* is muddying it — the half a mix report has always been missing, and
//! most of the work when a mix is wrong. Without it "87% of the energy is below
//! 250 Hz" is a true sentence whose only available response is to change four
//! instruments at once and re-bake.
//!
//! Same columns, same widths, same order as a section row, so the two tables
//! and the summary above them are read as one column of numbers.
//!
//! **One line per layer for the whole piece**, never per section: five
//! instruments over four sections is twenty rows in a report usually read as
//! "fine, carry on", and the per-section detail is already on the sum.

use scorsese_zimmer::level::Layer;

use super::table::columns;

/// One row per layer, named on the left where a section row carries its clock.
///
/// Rows are returned rather than printed and carry no leading indentation, for
/// the same reason [`super::sections`] does: the CLI nests them under a bake
/// and MCP hands them back as lines of a reply.
///
/// Empty in, empty out — a mix of one layer says nothing here, because a single
/// row under a one-line summary is that summary said twice.
pub fn layers(measured: &[Layer]) -> Vec<String> {
    let width = measured
        .iter()
        .map(|layer| layer.name.len())
        .max()
        .unwrap_or(0);
    measured
        .iter()
        .map(|layer| {
            format!(
                "{:<width$}  {}",
                layer.name,
                columns(&layer.level),
                width = width
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_zimmer::level::{Bands, Loudness, Span};

    fn layer(name: &str, mean: f64, low: f64) -> Layer {
        Layer {
            name: name.to_owned(),
            level: Span {
                label: None,
                from_seconds: 0.0,
                to_seconds: 10.0,
                loudness: Loudness {
                    peak_dbfs: Some(-3.0),
                    true_peak_dbfs: Some(-3.0),
                    mean_dbfs: Some(mean),
                },
                bands: Some(Bands {
                    low,
                    mid: 1.0 - low,
                    high: 0.0,
                }),
                correlation: Some(0.42),
            },
        }
    }

    /// The row names its layer and carries the same columns a section row does
    /// — which is the entire point of it: the two are read against each other.
    #[test]
    fn a_row_names_its_layer_and_says_where_its_energy_sits() {
        let rows = layers(&[layer("sub", -18.4, 0.96), layer("hat", -26.0, 0.02)]);
        assert_eq!(rows.len(), 2, "one row per layer");
        assert!(rows[0].starts_with("sub  "), "{}", rows[0]);
        assert!(rows[0].contains("mean  -18.4"), "{}", rows[0]);
        assert!(rows[0].contains("low  96%"), "{}", rows[0]);
        assert!(rows[1].contains("high   0%"), "{}", rows[1]);
        // A layer row carries the width figure too, which is what turns "this
        // mix collapses in mono" from a finding into an address.
        assert!(rows[0].contains("corr +0.42"), "{}", rows[0]);
    }

    /// Names are padded to the longest, so the columns line up down the table
    /// and a reader can scan one of them.
    #[test]
    fn names_are_padded_so_the_columns_line_up() {
        let rows = layers(&[layer("pad", -20.0, 0.5), layer("bell", -20.0, 0.5)]);
        let at = |row: &String| row.find("mean").expect("every row has a mean column");
        assert_eq!(at(&rows[0]), at(&rows[1]));
    }

    /// A mix of one layer has no table, the same way a piece of one section
    /// has no rows.
    #[test]
    fn nothing_measured_is_no_table() {
        assert!(layers(&[]).is_empty());
    }
}
