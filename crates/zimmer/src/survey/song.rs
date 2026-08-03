//! Reading one song document into the facts a survey reports.
//!
//! Nothing is rendered and nothing is measured — every number here is lifted
//! from the page. The one place that takes any thought is which notes count:
//! the arrangement can transpose a pattern and can silence a track for a
//! playing, so the register a piece actually reaches is not the register its
//! patterns were written in.

use std::collections::BTreeSet;

use super::{PitchClasses, Register, SongSurvey, TrackSurvey};
use crate::note::MIDI_RANGE;
use crate::patch::Patch;
use crate::song::shape::plan;
use crate::song::{PatchRef, PatchResolver, Song};

impl SongSurvey {
    /// Surveys one song, resolving any track that names its instrument rather
    /// than carrying it inline.
    ///
    /// A reference that cannot be resolved leaves that track's source and
    /// cutoff absent instead of failing: a survey is a report over a whole
    /// project, and one unreadable instrument should cost one field rather than
    /// the other five songs.
    pub fn of(name: &str, song: &Song, resolve: &dyn PatchResolver) -> Self {
        let (bpm, _) = plan(song);
        let (register, pitches) = played(song);
        Self {
            name: name.to_owned(),
            bpm,
            tracks: song
                .tracks
                .iter()
                .map(|track| {
                    let patch = instrument(&track.patch, resolve);
                    TrackSurvey {
                        name: track.name.clone(),
                        source: patch.as_ref().map(|patch| patch.source.kind()),
                        gain: track.gain,
                        cutoff: patch
                            .as_ref()
                            .and_then(|patch| patch.filter)
                            .map(|filter| filter.cutoff),
                    }
                })
                .collect(),
            register,
            pitches,
        }
    }

    /// Every source kind this song uses, each named once.
    pub(super) fn kinds(&self) -> BTreeSet<&'static str> {
        self.tracks
            .iter()
            .filter_map(|track| track.source)
            .collect()
    }
}

/// The patch behind a track, or nothing when its reference does not resolve.
fn instrument(reference: &PatchRef, resolve: &dyn PatchResolver) -> Option<Patch> {
    match reference {
        PatchRef::Inline(patch) => Some((**patch).clone()),
        PatchRef::Named(name) => resolve.resolve(name).ok(),
    }
}

/// The register and pitch classes of every note that actually sounds.
///
/// The arrangement's own transforms are applied, and clamped into the MIDI
/// range exactly as the renderer clamps them — so a pattern transposed past the
/// top of the keyboard is reported at the pitch it will be played at rather
/// than the one the arithmetic asked for. A note whose name does not parse is
/// skipped: refusing the document is [`Song::validate`]'s job, not a report's.
fn played(song: &Song) -> (Option<Register>, PitchClasses) {
    let mut register: Option<Register> = None;
    let mut pitches = PitchClasses::default();
    for entry in &song.arrangement {
        let Some(pattern) = song.patterns.get(entry.pattern()) else {
            continue;
        };
        for note in &pattern.notes {
            if !entry.plays(&note.track) {
                continue;
            }
            let Ok(written) = note.note.to_midi() else {
                continue;
            };
            let midi = (written + entry.transpose()).clamp(MIDI_RANGE.0, MIDI_RANGE.1);
            register =
                Some(register.map_or_else(|| Register::at(midi), |so_far| so_far.with(midi)));
            pitches.add(midi);
        }
    }
    (register, pitches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::InlineOnly;

    const SONG: &str = r#"{
      "bpm": 96,
      "tracks": [
        { "name": "lead", "gain": 0.9, "patch": {
            "source": { "kind": "karplus" },
            "amp": { "a": 0.01, "d": 0.2, "s": 0.4, "r": 0.3 },
            "filter": { "kind": "lowpass", "cutoff": 4600 } } },
        { "name": "pad", "gain": 0.4, "patch": {
            "source": { "kind": "osc_stack", "oscs": [{ "wave": "saw" }] },
            "amp": { "a": 0.2, "d": 0.1, "s": 0.8, "r": 0.5 } } }
      ],
      "patterns": { "a": { "beats": 4, "notes": [
        { "track": "lead", "note": "C4", "start": 0, "dur": 1 },
        { "track": "lead", "note": "E4", "start": 1, "dur": 1 },
        { "track": "pad", "note": "C2", "start": 0, "dur": 4 }] } },
      "arrangement": ["a", { "pattern": "a", "transpose": 7, "tracks": ["lead"] }]
    }"#;

    fn surveyed() -> SongSurvey {
        let song = Song::from_json(SONG).expect("the fixture parses");
        SongSurvey::of("theme", &song, &InlineOnly)
    }

    /// Every column of a per-song row comes off the page: what plays it, how
    /// loud it is written, and how it is filtered.
    #[test]
    fn a_song_reports_what_its_documents_say_it_is_made_of() {
        let survey = surveyed();
        assert_eq!(survey.bpm, 96.0);
        assert_eq!(survey.tracks[0].source, Some("karplus"));
        assert_eq!(survey.tracks[0].gain, 0.9);
        assert_eq!(survey.tracks[0].cutoff, Some(4600.0));
        assert_eq!(survey.tracks[1].source, Some("osc_stack"));
        assert_eq!(survey.tracks[1].cutoff, None, "no filter, no cutoff");
        assert_eq!(
            survey.loudest().map(|track| track.name.as_str()),
            Some("lead")
        );
    }

    /// The register is what is *played*: the second entry transposes the lead
    /// up a fifth and leaves the pad out, and both show.
    #[test]
    fn the_register_follows_the_arrangement_rather_than_the_patterns() {
        let survey = surveyed();
        let register = survey.register.expect("notes were played");
        assert_eq!(register.to_string(), "C2-B4");
        // C and E from the lead, C from the pad, and the fifth above each of
        // the lead's two — the pad's transposed C is absent because the second
        // entry does not play it.
        assert_eq!(survey.pitches.to_string(), "C E G B");
    }

    /// A song nobody wrote a note into has no register at all, rather than one
    /// at zero.
    #[test]
    fn a_song_with_no_notes_has_no_register() {
        let song =
            Song::from_json(r#"{ "bpm": 120, "tracks": [], "patterns": {}, "arrangement": [] }"#)
                .expect("parses");
        let survey = SongSurvey::of("empty", &song, &InlineOnly);
        assert_eq!(survey.register, None);
        assert!(survey.pitches.is_empty());
    }
}
