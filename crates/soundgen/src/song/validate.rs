//! What a song is refused for, before any samples are produced.
//!
//! A typo'd track or pattern name would otherwise be *silence in the middle of
//! a piece*, which is the failure mode that costs an agent a whole iteration to
//! even notice. So every name is resolved and every number is checked up front.

use super::timing::MAX_STRETCH;
use super::{ArrangementEntry, Note, Pattern, Song, Track};
use crate::error::SynthError;

impl Song {
    /// Rejects a song the renderer cannot honour, *before* any samples are
    /// produced — so a typo'd track or pattern name is a clear message rather
    /// than silence in the mix, which is the failure mode that would cost an
    /// agent a whole iteration to even notice.
    pub fn validate(&self) -> Result<(), SynthError> {
        if !(self.bpm.is_finite() && self.bpm > 0.0) {
            return Err(SynthError::BadBpm { bpm: self.bpm });
        }
        if self.tracks.is_empty() {
            return Err(SynthError::NoTracks);
        }
        if self.arrangement.is_empty() {
            return Err(SynthError::EmptyArrangement);
        }
        for entry in &self.arrangement {
            if !self.patterns.contains_key(entry.pattern()) {
                return Err(SynthError::UnknownPattern {
                    pattern: entry.pattern().to_owned(),
                });
            }
            self.check_entry(entry)?;
        }
        for (name, pattern) in &self.patterns {
            pattern.validate(name, &self.tracks)?;
        }
        self.check_shape()
    }

    /// One arrangement entry's transforms.
    ///
    /// A `tracks` filter naming no real track is the same typo as an
    /// arrangement naming no real pattern, and has the same consequence — an
    /// instrument that silently never plays — so it gets the same treatment.
    /// A transpose or a scale that is not a finite number is refused here
    /// too: a NaN reaches the mix as a whole song of silence, which is a long
    /// way from the field that caused it.
    fn check_entry(&self, entry: &ArrangementEntry) -> Result<(), SynthError> {
        let ArrangementEntry::Transformed(play) = entry else {
            return Ok(());
        };
        if let Some(transpose) = play.transpose
            && !transpose.is_finite()
        {
            return Err(SynthError::BadTranspose {
                pattern: play.pattern.clone(),
                transpose,
            });
        }
        if let Some(scale) = play.vel_scale
            && !(scale.is_finite() && scale >= 0.0)
        {
            return Err(SynthError::BadVelocityScale {
                pattern: play.pattern.clone(),
                scale,
            });
        }
        for wanted in play.tracks.iter().flatten() {
            if !self.tracks.iter().any(|track| &track.name == wanted) {
                return Err(SynthError::UnknownTrackFilter {
                    pattern: play.pattern.clone(),
                    track: wanted.clone(),
                });
            }
        }
        Ok(())
    }

    /// The length and level fields, checked here rather than at render time so
    /// a song that cannot be made to fit says so before anything is rendered.
    fn check_shape(&self) -> Result<(), SynthError> {
        if let Some(fade) = self.fade
            && !(fade.in_seconds.is_finite()
                && fade.in_seconds >= 0.0
                && fade.out_seconds.is_finite()
                && fade.out_seconds >= 0.0)
        {
            return Err(SynthError::BadFade {
                seconds: fade.in_seconds.max(fade.out_seconds),
            });
        }
        let Some(fit) = self.fit else {
            return Ok(());
        };
        if !(fit.seconds.is_finite() && fit.seconds > 0.0) {
            return Err(SynthError::BadFitSeconds {
                seconds: fit.seconds,
            });
        }
        // A stretch beyond the bound would deliver something nobody would use,
        // so it is refused with the tempo it would have needed — which is the
        // number a caller needs to decide what to do instead.
        if let Some(ratio) = super::shape::stretch_ratio(self, fit)
            && ratio.abs() > MAX_STRETCH
        {
            return Err(SynthError::StretchTooFar {
                bpm: self.bpm,
                needed: self.bpm * (1.0 + ratio),
                limit: MAX_STRETCH,
            });
        }
        Ok(())
    }
}

impl Pattern {
    /// Checks one pattern's slot length and every note in it.
    fn validate(&self, name: &str, tracks: &[Track]) -> Result<(), SynthError> {
        if !(self.beats.is_finite() && self.beats > 0.0) {
            return Err(SynthError::BadPatternBeats {
                pattern: name.to_owned(),
                beats: self.beats,
            });
        }
        for (index, note) in self.notes.iter().enumerate() {
            note.validate(name, index, tracks)?;
        }
        Ok(())
    }
}

impl Note {
    /// Checks that one note names a real track, starts somewhere, lasts for
    /// some time, and has a pitch that parses.
    fn validate(&self, pattern: &str, index: usize, tracks: &[Track]) -> Result<(), SynthError> {
        let pattern = || pattern.to_owned();
        if !tracks.iter().any(|track| track.name == self.track) {
            return Err(SynthError::UnknownTrack {
                pattern: pattern(),
                index,
                track: self.track.clone(),
            });
        }
        if !(self.start.is_finite() && self.start >= 0.0) {
            return Err(SynthError::BadNoteStart {
                pattern: pattern(),
                index,
                start: self.start,
            });
        }
        if !(self.dur.is_finite() && self.dur > 0.0) {
            return Err(SynthError::BadNoteDuration {
                pattern: pattern(),
                index,
                dur: self.dur,
            });
        }
        self.note.to_midi()?;
        Ok(())
    }
}
