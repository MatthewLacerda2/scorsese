//! Which layer of a mix owns the energy.
//!
//! [`super::profile`] cuts a signal by **time**; this cuts it by **part**. They
//! answer questions that look alike and are not: a section row says *when* a
//! piece is muddy, and a layer row says *which of the five things playing* is
//! muddying it. A mix report without the second is a diagnosis with no address
//! — "87% of the energy is below 250 Hz" is correct, actionable by nobody, and
//! is fixed by changing four instruments at once and hoping.
//!
//! A layer is measured with the same two meters, over the same window, as the
//! sum it sits under, which is what lets the rows be read as a column: the
//! numbers are comparable because they are the same numbers.
//!
//! **Including how wide it is**, which is the column that most needs an
//! address. A mix that reads negative — cancelling, and about to collapse the
//! moment anything folds it to mono — is a finding with nowhere to go until
//! something says *which of the five things playing* is doing it; that is the
//! same argument this file exists for. The column is free besides: a layer row
//! and a section row are printed by one formatter precisely so the two can be
//! read as one column of numbers, and suppressing it here would take a flag
//! and would break that.
//!
//! **Whole only, never sectioned.** One line per layer for the whole piece.
//! Five instruments over four sections is twenty rows in a report that is
//! usually read as "fine, carry on", and the per-section detail is already on
//! the sum, where it is read against the arrangement rather than against four
//! other instruments.

use super::bands::BandMeter;
use super::meter::Meter;
use super::profile::Span;

/// One layer of a mix, measured on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// What the document calls it — in a song, a track's name.
    pub name: String,
    /// How it came out: the same statistics, in the same columns, as the
    /// summary above it.
    ///
    /// A [`Span`] rather than a type of its own precisely because it must line
    /// up with the rows around it — a layer measured differently from the sum
    /// could not be compared with it, which is the only thing anyone does with
    /// one. Its `label` is `None`; the name a reader wants is beside it.
    pub level: Span,
}

impl Layer {
    /// Measures `samples` whole, as one named span.
    ///
    /// The samples are handed over complete rather than fed a run at a time,
    /// because a layer only exists at the moment a mix is summed — there is no
    /// streaming caller for it, and the buffer is in hand and about to be
    /// dropped.
    pub fn of(name: impl Into<String>, samples: &[f32], channels: usize, rate: u32) -> Self {
        let channels = channels.max(1);
        let mut level = Meter::new(channels);
        let mut bands = BandMeter::new(channels, rate);
        level.feed(samples);
        bands.feed(samples);
        Self {
            name: name.into(),
            level: Span {
                label: None,
                from_seconds: 0.0,
                to_seconds: seconds(samples.len() / channels, rate),
                loudness: level.finish(),
                bands: bands.finish(),
                correlation: level.correlation(),
            },
        }
    }
}

/// How long a count of frames runs, in seconds.
fn seconds(frames: usize, rate: u32) -> f64 {
    if rate == 0 {
        return 0.0;
    }
    frames as f64 / f64::from(rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A layer is measured exactly as the whole file is, which is the only
    /// reason its row can be read against the summary above it.
    #[test]
    fn a_layer_reports_the_same_statistics_as_a_whole_signal() {
        let square: Vec<f32> = (0..1_000)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let layer = Layer::of("sub", &square, 1, 1_000);
        assert_eq!(layer.name, "sub");
        let mean = layer.level.loudness.mean_dbfs.expect("it is not silent");
        assert!(
            (mean + 6.0).abs() < 0.05,
            "half scale is -6 dBFS, not {mean}"
        );
        assert!((layer.level.to_seconds - 1.0).abs() < 1e-9);
    }

    /// A track that never played is said to be silent rather than left out: a
    /// missing row reads as an oversight, and "the arp is not in this mix" is a
    /// finding.
    #[test]
    fn a_layer_that_never_played_is_silent_rather_than_absent() {
        let layer = Layer::of("arp", &[], 1, 48_000);
        assert!(layer.level.loudness.is_silent());
        assert!(layer.level.bands.is_none(), "a silence has no balance");
    }
}
