//! What a repeating layer sounds like: its passes, the last one cut, and a
//! glide across the seam between two.

use crate::common::saw_patch;
use crate::common::songs::{note, played};
use scorsese_zimmer::song::{Articulation, InlineOnly, Note, PatchRef, Pattern, TempoChange};
use scorsese_zimmer::{NoteAt, SynthError, render_song};

use super::super::setup::render;
use super::{band, layered, repeating, written_once};

#[test]
fn a_repeating_layer_equals_its_passes_written_out_as_one_pattern() {
    let mut by_hand = band();
    let mut both = by_hand.patterns["solo"].clone();
    for pass in 0..4 {
        let at = 2.0 * pass as f32;
        both.notes.extend(played(vec![
            note("bass", "E2", at, 0.5),
            note("bass", "B2", at + 1.0, 0.5),
        ]));
    }
    by_hand.patterns.insert("both".to_owned(), both);
    by_hand.arrangement = vec!["both".into()];

    assert_eq!(written_once().arrangement_beats(), 8.0);
    assert_eq!(render(&written_once()), render(&by_hand));
}

/// A slot that is not a whole number of passes cuts the last one: a note that
/// starts before the slot ends plays, one that would start at or after it
/// does not.
#[test]
fn the_last_pass_is_cut_where_the_slot_ends() {
    let mut cut = band();
    cut.patterns.get_mut("solo").expect("defined above").beats = 3.0;
    cut.patterns
        .get_mut("solo")
        .expect("defined above")
        .notes
        .truncate(3);
    cut.arrangement = vec![layered(vec!["solo".into(), repeating("verse")])];

    let mut by_hand = band();
    let mut short = by_hand.patterns["solo"].clone();
    short.beats = 3.0;
    short.notes.truncate(3);
    short.notes.extend(played(vec![
        note("bass", "E2", 0.0, 0.5),
        note("bass", "B2", 1.0, 0.5),
        note("bass", "E2", 2.0, 0.5),
    ]));
    by_hand.patterns.insert("short".to_owned(), short);
    by_hand.arrangement = vec!["short".into()];

    assert_eq!(cut.arrangement_beats(), 3.0);
    assert_eq!(render(&cut), render(&by_hand));
}

/// A glide at the top of a pass slides from where the pass before it left
/// the hand, across the seam — as it does from one entry to the next.
#[test]
fn a_glide_slides_across_the_seam_between_passes() {
    let gliding = |mut it: Note| {
        it.articulation = Some(Articulation::Glide);
        it
    };
    let mut once = band();
    // Pitched, or a slide would be a slide of noise and sound like none.
    once.tracks[0].patch = PatchRef::Inline(Box::new(saw_patch()));
    once.patterns.insert(
        "line".to_owned(),
        Pattern {
            beats: 2.0,
            notes: played(vec![
                gliding(note("bass", "C2", 0.0, 0.5)),
                note("bass", "G2", 1.0, 0.5),
            ]),
        },
    );
    once.arrangement = vec![layered(vec!["solo".into(), repeating("line")])];

    let mut by_hand = once.clone();
    let mut both = by_hand.patterns["solo"].clone();
    for pass in 0..4 {
        let at = 2.0 * pass as f32;
        both.notes.extend(played(vec![
            gliding(note("bass", "C2", at, 0.5)),
            note("bass", "G2", at + 1.0, 0.5),
        ]));
    }
    by_hand.patterns.insert("both".to_owned(), both);
    by_hand.arrangement = vec!["both".into()];

    assert_eq!(render(&once), render(&by_hand));
}

/// A note refused in a later pass is named where its pattern writes it, not
/// where the pass put it — the pattern is the only place to find it.
#[test]
fn a_refusal_in_a_later_pass_names_the_beat_its_pattern_writes() {
    let mut slowing = band();
    // Four beats is two seconds here, and eighty from beat 8 on.
    slowing.tempo = vec![TempoChange {
        beat: 8.0,
        bpm: 3.0,
        ramp: false,
    }];
    slowing
        .patterns
        .get_mut("solo")
        .expect("defined above")
        .beats = 16.0;
    let verse = slowing.patterns.get_mut("verse").expect("the fixture's");
    verse.notes = played(vec![note("bass", "E2", 0.5, 4.0)]);
    slowing.arrangement = vec![layered(vec!["solo".into(), repeating("verse")])];

    let refused = render_song(&slowing, &InlineOnly).expect_err("refused");
    let SynthError::NoteTooLong { at, .. } = &refused else {
        panic!("refused for the wrong reason: {refused:?}")
    };
    assert_eq!(
        at.as_ref(),
        Some(&NoteAt {
            pattern: "verse".to_owned(),
            track: "bass".to_owned(),
            start: 0.5,
        })
    );
}
