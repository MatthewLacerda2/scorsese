//! Where a song's sections fall in the piece it renders to.
//!
//! The arrangement already says this — it is a list of patterns, and every
//! pattern knows how many beats its slot is. All that is left is to turn beats
//! into seconds, which is the one conversion a song defers to render time.
//!
//! It matters that the rows of a report are *these* boundaries rather than an
//! arbitrary grid: "the second chorus is the quiet one" is a finding an author
//! can act on, and "seconds 24 to 32 are quiet" is a fact they then have to
//! look up. A fixed interval is the fallback for a signal whose document does
//! not say — see [`crate::level::profile`].

use super::Song;
use super::shape::plan;
use crate::level::Cut;

/// Every arrangement entry's end, in seconds of the rendered piece.
///
/// Taken from the *planned* tempo and pass count rather than from the written
/// ones, because `fit` can change both: a song stretched to land on a cut plays
/// at a tempo the document never states, and one looped to fill a bed plays its
/// arrangement several times over. Reporting the written positions would put
/// every row in the wrong place for exactly the songs whose length someone
/// cared about.
///
/// The last boundary is where the arrangement ends, not where the audio does —
/// a piece rings out past its final beat, and that ring-out is left to the
/// profiler's fixed-interval fallback rather than being folded into the last
/// pattern, which did not play it.
pub fn of(song: &Song) -> Vec<Cut> {
    let (bpm, passes) = plan(song);
    if bpm <= 0.0 {
        return Vec::new();
    }
    let beat = f64::from(60.0 / bpm);
    let mut end = 0.0;
    let mut cuts = Vec::new();
    for entry in song
        .arrangement
        .iter()
        .cycle()
        .take(song.arrangement.len() * passes as usize)
    {
        let Some(pattern) = song.patterns.get(entry.pattern()) else {
            continue;
        };
        end += f64::from(pattern.beats) * beat;
        cuts.push(Cut {
            label: entry.pattern().to_owned(),
            end_seconds: end,
        });
    }
    cuts
}
