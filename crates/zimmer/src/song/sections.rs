//! Where a song's sections fall in the piece it renders to.
//!
//! The arrangement already says this — it is a list of patterns, marked into
//! named sections, and every pattern knows how many beats its slot is. All that is left is to turn beats
//! into seconds, which is the one conversion a song defers to render time.
//!
//! It matters that the rows of a report are *these* boundaries rather than an
//! arbitrary grid: "the second chorus is the quiet one" is a finding an author
//! can act on, and "seconds 24 to 32 are quiet" is a fact they then have to
//! look up. A fixed interval is the fallback for a signal whose document does
//! not say — see [`crate::level::profile`].

use super::Song;
use super::clock::Clock;
use super::shape::plan;
use crate::level::Cut;

/// Every section's end, in seconds of the rendered piece: one per named
/// section ([`Marker`](super::Marker)), and one per entry outside any.
///
/// A named section is a run of entries, ending where its last one ends and
/// labelled with its name: a song written as many short layered entries is
/// reported in its verse and chorus, not in bars. An entry before the first
/// marker — or in a song with none — is a section of its own, labelled with
/// its patterns; a layered one included, because a solo over a groove is one
/// stretch, labelled with both.
///
/// Taken from the *planned* tempo and pass count rather than from the written
/// ones, because `fit` can change both: a song stretched to land on a cut plays
/// at a tempo the document never states, and one looped to fill a bed plays its
/// arrangement several times over. Reporting the written positions would put
/// every row in the wrong place for exactly the songs whose length someone
/// cared about. Each pass closes the section it ends in, so a loop repeats
/// the rows.
///
/// The last boundary is where the arrangement ends, not where the audio does —
/// a piece rings out past its final beat, and that ring-out is left to the
/// profiler's fixed-interval fallback rather than being folded into the last
/// pattern, which did not play it.
fn whole(song: &Song) -> Vec<Cut> {
    let (clock, passes) = plan(song);
    if clock.bpm() <= 0.0 {
        return Vec::new();
    }
    // Beats are summed and converted, rather than seconds summed, because that
    // is how the renderer walks the arrangement: a boundary is then the exact
    // number a note written on that downbeat was placed at — tempo map and
    // all, since the clock carries it.
    let mut beats = 0.0f32;
    let mut cuts = Vec::new();
    for _ in 0..passes {
        // The named section open at this point of the pass, and whether any
        // entry of it has played yet — a marker with nothing after it reports
        // nothing, as a stretch of no length would.
        let mut open: Option<(&str, bool)> = None;
        for entry in &song.arrangement {
            if let Some(name) = entry.marker() {
                close(&mut cuts, open.take(), &clock, beats);
                open = Some((name, false));
                continue;
            }
            let Some(slot) = song.slot_beats(entry) else {
                continue;
            };
            beats += slot;
            match &mut open {
                Some((_, played)) => *played = true,
                None => cuts.push(Cut {
                    label: entry.label(),
                    end_seconds: f64::from(clock.seconds(beats)),
                }),
            }
        }
        close(&mut cuts, open, &clock, beats);
    }
    cuts
}

/// Ends the named section `open` at `beats`, if any of it played.
fn close(cuts: &mut Vec<Cut>, open: Option<(&str, bool)>, clock: &Clock, beats: f32) {
    if let Some((name, true)) = open {
        cuts.push(Cut {
            label: name.to_owned(),
            end_seconds: f64::from(clock.seconds(beats)),
        });
    }
}

impl Song {
    /// Where each section of the arrangement ends, in seconds of the piece
    /// this song renders to — worked out from the document, without rendering
    /// it. A section is a run of entries a marker names, or one entry outside
    /// any.
    ///
    /// The same boundaries a bake's section rows are cut at, and through the
    /// same conversion the notes are placed with, so a clip put on one of
    /// them lands on the music. Public for the caller that has a bake already
    /// on disk and still wants to know where its sections are: those are
    /// arithmetic on the document rather than a measurement, so answering
    /// costs nothing and needs no render.
    ///
    /// One per section per pass, in order; each section starts where
    /// the one before it ends, and the first at zero. The ring-out past the
    /// last one is not a section — nothing in the arrangement played it.
    pub fn sections(&self) -> Vec<Cut> {
        whole(self)
    }

    /// The named section arrangement entry `entry` is in: the last marker
    /// before it. `None` before the first marker, and for a marker itself.
    pub fn section_of(&self, entry: usize) -> Option<&str> {
        self.arrangement
            .get(..entry)?
            .iter()
            .rev()
            .find_map(|it| it.marker())
            .filter(|_| {
                self.arrangement
                    .get(entry)
                    .is_some_and(|it| it.marker().is_none())
            })
    }
}

/// The same boundaries, measured from `start_seconds` into the piece — which
/// is where an excerpt's rows begin.
///
/// A section that has already finished by then is dropped rather than kept at
/// a negative end: it is not in what was rendered, and a row for it would send
/// its reader to look for music that is not in the file. The one it lands in
/// the middle of keeps its name and its own end, so a window opening halfway
/// through a chorus is reported as being in that chorus. Nothing clips the far
/// end, because nothing has to — the profiler already runs out of boundaries
/// before the buffer does, which is how a ring-out is reported.
pub(crate) fn of(song: &Song, start_seconds: f64) -> Vec<Cut> {
    whole(song)
        .into_iter()
        .map(|cut| Cut {
            end_seconds: cut.end_seconds - start_seconds,
            ..cut
        })
        .filter(|cut| cut.end_seconds > 0.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::{Pattern, Song};
    use std::collections::BTreeMap;

    /// Two named blocks, four beats each, at a tempo where a beat is half a
    /// second — so a boundary is a round number and a shift is readable.
    fn two_sections() -> Song {
        let mut patterns = BTreeMap::new();
        for name in ["verse", "chorus"] {
            patterns.insert(
                name.to_owned(),
                Pattern {
                    beats: 4.0,
                    notes: Vec::new(),
                },
            );
        }
        Song {
            bpm: 120.0,
            tempo: vec![],
            seed: 0,
            key: None,
            tracks: Vec::new(),
            patterns,
            arrangement: vec!["verse".into(), "chorus".into()],
            swing: 0.0,
            humanize: None,
            fx: Vec::new(),
            automation: Vec::new(),
            fit: None,
            anchors: Vec::new(),
            fade: None,
            tail: None,
        }
    }

    fn ends(cuts: &[Cut]) -> Vec<(String, f64)> {
        cuts.iter()
            .map(|cut| (cut.label.clone(), cut.end_seconds))
            .collect()
    }

    #[test]
    fn a_whole_piece_is_reported_from_its_own_start() {
        assert_eq!(
            ends(&of(&two_sections(), 0.0)),
            vec![("verse".to_owned(), 2.0), ("chorus".to_owned(), 4.0)]
        );
    }

    /// A window opening inside the verse keeps the verse — with the time that
    /// is left of it — and every later boundary moves with it.
    #[test]
    fn a_window_moves_every_boundary_back_by_where_it_opened() {
        assert_eq!(
            ends(&of(&two_sections(), 1.5)),
            vec![("verse".to_owned(), 0.5), ("chorus".to_owned(), 2.5)]
        );
    }

    /// A window opening exactly on a boundary starts in the section *after*
    /// it. A row for the one that has just finished would be a stretch of no
    /// length, sending its reader to look for music the file does not carry.
    #[test]
    fn a_section_that_has_already_ended_gets_no_row() {
        assert_eq!(
            ends(&of(&two_sections(), 2.0)),
            vec![("chorus".to_owned(), 2.0)]
        );
    }
}
