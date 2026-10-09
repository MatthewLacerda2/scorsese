//! Cutting and timing captions from a line's words.

mod disk;
mod fixture;

use crate::ClipId;
use crate::captions::{Chunking, chunks, time};
use crate::words::Placed;

/// Words said one after another, `0.3` s each with no gap.
fn said(text: &str) -> Vec<Placed> {
    text.split(' ')
        .enumerate()
        .map(|(i, word)| Placed {
            name: word.to_lowercase(),
            text: word.to_owned(),
            start: i as f64 * 0.3,
            end: i as f64 * 0.3 + 0.3,
        })
        .collect()
}

fn texts(line: &str, chunking: Chunking) -> Vec<String> {
    chunks(&ClipId::new("vo"), &said(line), chunking)
        .into_iter()
        .map(|chunk| chunk.text)
        .collect()
}

#[test]
fn a_sentence_end_always_breaks() {
    assert_eq!(
        texts("Pare. Seus dados valem ouro!", Chunking::DEFAULT),
        ["Pare.", "Seus dados valem ouro!"]
    );
}

#[test]
fn a_long_clause_breaks_at_its_comma_and_never_runs_past_the_bound() {
    let line = "When the numbers finally add up, every team in the building wants the same report";
    let cut = texts(line, Chunking::DEFAULT);
    assert_eq!(cut[0], "When the numbers finally add up,");
    assert!(
        cut.iter().all(|chunk| chunk.chars().count() <= 36),
        "{cut:?}"
    );
    assert_eq!(cut.join(" "), line, "no word lost, none reordered");
}

#[test]
fn a_comma_too_near_either_end_is_not_a_break() {
    let cut = texts(
        "Yes, we measured every single one of these things carefully",
        Chunking {
            max_chars: 30,
            ..Chunking::DEFAULT
        },
    );
    assert_eq!(cut[0], "Yes, we measured every single");
}

#[test]
fn a_pause_breaks_without_punctuation() {
    let mut words = said("first part second part");
    for word in &mut words[2..] {
        word.start += 1.0;
        word.end += 1.0;
    }
    let cut: Vec<String> = chunks(&ClipId::new("vo"), &words, Chunking::DEFAULT)
        .into_iter()
        .map(|chunk| chunk.text)
        .collect();
    assert_eq!(cut, ["first part", "second part"]);
}

#[test]
fn a_caption_leads_its_word_and_stays_until_the_next_or_a_hold() {
    let mut words = said("One. Two. Three.");
    // Three comes two seconds after Two ends: too long to wait through.
    words[2].start += 2.0;
    words[2].end += 2.0;
    let timed = time(
        chunks(&ClipId::new("vo"), &words, Chunking::DEFAULT),
        Chunking::DEFAULT,
    );
    let at: Vec<(f64, f64)> = timed.iter().map(|c| (c.start, c.end)).collect();
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    // One is said at 0 — it cannot arrive before the timeline does.
    assert!(near(at[0].0, 0.0) && near(at[0].1, 0.3 - 0.07), "{at:?}");
    // Two stays its hold past its word, since Three is far off.
    assert!(near(at[1].0, 0.23) && near(at[1].1, 0.6 + 0.5), "{at:?}");
    assert!(
        near(at[2].0, 2.6 - 0.07) && near(at[2].1, 2.9 + 0.5),
        "{at:?}"
    );
}

#[test]
fn captions_from_two_lines_never_share_the_screen() {
    let a = chunks(&ClipId::new("a"), &said("Alpha beta."), Chunking::DEFAULT);
    let mut b_words = said("Gamma.");
    b_words[0].start = 0.4;
    b_words[0].end = 0.7;
    let b = chunks(&ClipId::new("b"), &b_words, Chunking::DEFAULT);
    let timed = time([a, b].concat(), Chunking::DEFAULT);
    assert_eq!(timed[0].line.as_str(), "a");
    assert!(timed[0].end <= timed[1].start, "{timed:?}");
}

#[test]
fn the_bound_counts_characters_and_the_spaces_between_words() {
    let bound = |max_chars| {
        texts(
            "aaa bb ccc",
            Chunking {
                max_chars,
                ..Chunking::DEFAULT
            },
        )
    };
    assert_eq!(bound(10), ["aaa bb ccc"], "exactly at the bound fits");
    assert_eq!(bound(9), ["aaa bb", "ccc"]);
    assert_eq!(bound(5), ["aaa", "bb", "ccc"]);
}

#[test]
fn a_comma_inside_a_full_caption_is_where_it_breaks() {
    let cut = |line| {
        texts(
            line,
            Chunking {
                max_chars: 24,
                ..Chunking::DEFAULT
            },
        )
    };
    assert_eq!(
        cut("One two three, four five six"),
        ["One two three,", "four five six"]
    );
    assert_eq!(
        cut("We looked, and then we found it"),
        ["We looked,", "and then we found it"]
    );
}

#[test]
fn a_short_silence_is_waited_through_and_a_long_one_is_not() {
    let mut words = said("One. Two.");
    // Two arrives 0.8 s after One ends: inside the bridge, past the hold.
    words[1].start = 1.1;
    words[1].end = 1.4;
    let timed = time(
        chunks(&ClipId::new("vo"), &words, Chunking::DEFAULT),
        Chunking::DEFAULT,
    );
    assert!((timed[0].end - (1.1 - 0.07)).abs() < 1e-9, "{timed:?}");
}
