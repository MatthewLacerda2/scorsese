//! Where each note of a layer falls in its slot — once for a layer as
//! written, once per pass for one that [repeats](super::Play::repeat).
//!
//! The one place a repeat is expanded, so the renderer, the survey, the
//! length cap and the MIDI file cannot disagree about which notes a cut pass
//! keeps. A pass is **the pattern's notes again, moved later**, in the order
//! the pattern is walked: so each pass takes consecutive places in the walk,
//! exactly as the same pattern in consecutive entries does.

use super::{ArrangementEntry, Layer, Note, Song};
use crate::error::SynthError;

/// Every note `layer` plays in a slot of `slot` beats, as `(index, offset)`:
/// note `index` of the pattern's `starts`, played `offset` beats after the
/// slot's first beat.
///
/// A layer that does not repeat is every note once, at offset zero, whatever
/// the slot — a short layer rests, and a long one sets the slot. One that
/// repeats starts again every `beats` and keeps a note only if it starts
/// before the slot ends.
///
/// A pattern of no length, or a slot of none to fill, is one pass: a walk that
/// tolerates an unvalidated document must still end.
pub(crate) fn passes(
    layer: &Layer,
    starts: impl Fn(usize) -> f32,
    len: usize,
    beats: f32,
    slot: f32,
) -> Vec<(usize, f32)> {
    if !layer.repeats() {
        return (0..len).map(|index| (index, 0.0)).collect();
    }
    let count = if beats > 0.0 && (slot / beats).is_finite() {
        (slot / beats).ceil().max(1.0) as usize
    } else {
        1
    };
    (0..count)
        .flat_map(|pass| {
            let offset = pass as f32 * beats;
            (0..len).map(move |index| (index, offset))
        })
        .filter(|&(index, offset)| offset + starts(index) < slot)
        .collect()
}

/// The notes `layer` plays in a slot of `slot` beats: `notes` as written, or
/// every pass of them moved to where it falls — the form the renderer walks.
pub(crate) fn tiled(layer: &Layer, notes: &[Note], beats: f32, slot: f32) -> Vec<Note> {
    passes(layer, |index| notes[index].start, notes.len(), beats, slot)
        .into_iter()
        .map(|(index, offset)| Note {
            start: notes[index].start + offset,
            ..notes[index].clone()
        })
        .collect()
}

/// Every entry that repeats a layer has something beside it that sets how
/// long it is.
pub(crate) fn check(song: &Song) -> Result<(), SynthError> {
    for (index, entry) in song.arrangement.iter().enumerate() {
        match entry {
            ArrangementEntry::Single(layer) if layer.repeats() => {
                return Err(SynthError::RepeatAlone {
                    index,
                    pattern: layer.pattern().to_owned(),
                });
            }
            ArrangementEntry::Layered(layered)
                if !layered.layers.is_empty() && layered.layers.iter().all(Layer::repeats) =>
            {
                return Err(SynthError::EveryLayerRepeats { index });
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::Play;

    fn layer(repeat: bool) -> Layer {
        Layer::Transformed(Play {
            pattern: "beat".to_owned(),
            transpose: None,
            transpose_degrees: None,
            vel_scale: None,
            tracks: None,
            repeat,
        })
    }

    /// Two notes, at 0 and 1, of a two-beat pattern.
    fn starts(index: usize) -> f32 {
        index as f32
    }

    #[test]
    fn a_layer_that_does_not_repeat_plays_once_whatever_the_slot() {
        assert_eq!(
            passes(&layer(false), starts, 2, 2.0, 7.0),
            [(0, 0.0), (1, 0.0)]
        );
        assert_eq!(
            passes(&layer(false), starts, 2, 2.0, 1.0),
            [(0, 0.0), (1, 0.0)]
        );
    }

    /// Three beats of a two-beat pattern: the second pass keeps the note on
    /// beat 2 and drops the one that would start on beat 3, where the slot
    /// ends.
    #[test]
    fn a_repeating_layer_plays_until_the_slot_ends_and_no_further() {
        assert_eq!(
            passes(&layer(true), starts, 2, 2.0, 3.0),
            [(0, 0.0), (1, 0.0), (0, 2.0)]
        );
        assert_eq!(
            passes(&layer(true), starts, 2, 2.0, 4.0),
            [(0, 0.0), (1, 0.0), (0, 2.0), (1, 2.0)]
        );
    }

    /// A repeating layer longer than its slot is cut in its first pass.
    #[test]
    fn a_repeating_layer_longer_than_its_slot_is_cut_in_its_first_pass() {
        assert_eq!(passes(&layer(true), starts, 2, 2.0, 1.0), [(0, 0.0)]);
    }

    #[test]
    fn a_pattern_of_no_length_is_one_pass() {
        assert_eq!(
            passes(&layer(true), starts, 2, 0.0, 4.0),
            [(0, 0.0), (1, 0.0)]
        );
    }
}
