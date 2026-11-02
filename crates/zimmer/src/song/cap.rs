//! The longest a song may ask to be rendered: [`MAX_SONG_SECONDS`].
//!
//! A song is rendered into one buffer, allocated whole, so its length is its
//! memory: nothing else in a document stops a pattern of a million beats, a
//! `loop` fit a million seconds long, or a note placed a million beats in.
//! [`Song::validate`] refuses each before a sample exists, by working out
//! where the last thing in one pass of the arrangement ends — a note that
//! rings past its pattern included — at the tempo it is played at, and taking
//! the target of a `fit` when it is longer. More passes than one exist only
//! under a `fit`, and they come to its target: a `loop` is cut there, a
//! `stretch` lands there, and each pass before is shorter than it.
//!
//! A note's own release and its effect tail are already bounded where a note
//! is (`NoteTooLong`), so they are not counted here.

use std::collections::HashMap;

use super::Song;
use super::key::Key;
use super::shape::{fitted, plan};
use crate::error::SynthError;

/// The longest a song may be rendered, in seconds: ten minutes, longer than
/// any bed a video plays under one scene, and well short of a buffer that
/// hurts the machine it is baked on.
pub const MAX_SONG_SECONDS: f32 = 600.0;

/// Refused when `song`, read in `key`, would render past
/// [`MAX_SONG_SECONDS`]. Called last by `validate`, on a song otherwise valid.
pub(super) fn check(song: &Song, key: Option<&Key>) -> Result<(), SynthError> {
    let (clock, _) = plan(song);
    let once = clock.seconds(last_beat(song, key)?);
    let seconds = fitted(song).map_or(once, |(_, target)| once.max(target));
    if seconds.is_finite() && seconds <= MAX_SONG_SECONDS {
        return Ok(());
    }
    Err(SynthError::SongTooLong {
        seconds,
        limit: MAX_SONG_SECONDS,
    })
}

/// The beat the last thing in one pass ends on: each slot as far as its
/// furthest-reaching layer reaches.
fn last_beat(song: &Song, key: Option<&Key>) -> Result<f32, SynthError> {
    let mut reach = HashMap::with_capacity(song.patterns.len());
    for (name, pattern) in &song.patterns {
        let notes = pattern.voices(key)?;
        let end = notes
            .iter()
            .map(|note| note.start + note.dur)
            .fold(pattern.beats, f32::max);
        reach.insert(name.as_str(), end);
    }
    let (mut cursor, mut end) = (0.0f32, 0.0f32);
    for entry in &song.arrangement {
        for layer in entry.layers() {
            if let Some(reaches) = reach.get(layer.pattern()) {
                end = end.max(cursor + reaches);
            }
        }
        cursor += song.slot_beats(entry).unwrap_or(0.0);
    }
    Ok(end)
}
