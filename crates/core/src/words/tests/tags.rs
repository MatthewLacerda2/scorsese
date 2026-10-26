//! An `expressive` line's audio tags are directions, not words (#1061).

use crate::SpeechModel;
use crate::words::{Word, Words};

/// A line as Eleven v3 times it: every character of the prompt as sent,
/// tags included, each word spread over `spans` and a space between.
fn read(line: &str, spans: &[(f64, f64)]) -> Vec<(char, f64, f64)> {
    let mut characters = Vec::new();
    for (word, &(start, end)) in line.split(' ').zip(spans) {
        if !characters.is_empty() {
            characters.push((' ', start, start));
        }
        let each = (end - start) / word.chars().count() as f64;
        for (i, c) in word.chars().enumerate() {
            let at = start + each * i as f64;
            characters.push((c, at, at + each));
        }
    }
    characters
}

/// The first line of `every-morning.scor`, as v3 returned it.
fn gruff() -> Vec<(char, f64, f64)> {
    let spans = [
        (0.00, 0.06),
        (0.07, 0.08),
        (0.08, 0.10),
        (0.11, 0.14),
        (0.15, 0.24),
        (0.32, 1.60),
    ];
    read("[gruff, not looking up] We're closed.", &spans)
}

fn texts(words: &Words) -> Vec<&str> {
    words.words.iter().map(|w| w.text.as_str()).collect()
}

#[test]
fn an_expressive_line_folds_without_its_tags() {
    let words = Words::from_reading(gruff(), SpeechModel::Expressive);
    assert_eq!(texts(&words), ["We're", "closed."]);
    assert!((words.words[0].start - 0.15).abs() < 1e-9);
    let sighs = read("Okay. [sighs]", &[(0.0, 0.4), (0.5, 1.5)]);
    let words = Words::from_reading(sighs, SpeechModel::Expressive);
    assert_eq!(
        texts(&words),
        ["Okay."],
        "a tag at the end stretches nothing"
    );
    assert!((words.words[0].end - 0.4).abs() < 1e-9);
    let glued = read("[sighs]Okay.", &[(0.0, 1.2)]);
    let words = Words::from_reading(glued, SpeechModel::Expressive);
    assert_eq!(texts(&words), ["Okay."]);
    assert!(words.words[0].start > 0.69, "it starts where it is said");
}

#[test]
fn an_empty_tag_is_dropped_and_what_follows_it_is_kept() {
    let empty = read("Okay. [] Fine.", &[(0.0, 0.4), (0.5, 0.6), (0.7, 1.2)]);
    let words = Words::from_reading(empty, SpeechModel::Expressive);
    assert_eq!(texts(&words), ["Okay.", "Fine."]);
}

#[test]
fn brackets_are_words_on_the_models_that_read_them() {
    for model in [SpeechModel::Standard, SpeechModel::Fast] {
        let words = Words::from_reading(gruff(), model);
        assert_eq!(texts(&words)[0], "[gruff,", "{model:?}");
        assert_eq!(words.clone().spoken_by(model), words);
    }
    let open = read("[an aside that never closes", &[(0.0, 0.5); 5]);
    let words = Words::from_reading(open, SpeechModel::Expressive);
    assert_eq!(texts(&words)[0], "[an", "an unclosed bracket is no tag");
}

#[test]
fn a_line_folded_with_its_tags_loses_them_when_read_back() {
    let folded = Words::from_characters(gruff());
    assert_eq!(texts(&folded)[0], "[gruff,");
    let spoken = folded.spoken_by(SpeechModel::Expressive);
    assert_eq!(
        spoken,
        Words::from_reading(gruff(), SpeechModel::Expressive)
    );
    let said = |text: &str, start: f64| Word {
        text: text.into(),
        start,
        end: start + 0.5,
    };
    let glued = Words {
        words: vec![said("Fine.", 0.0), said("[sighs]Okay.", 1.0)],
    };
    let spoken = glued.spoken_by(SpeechModel::Expressive);
    assert_eq!(texts(&spoken), ["Fine.", "Okay."]);
    assert_eq!(spoken.words[1].start, 1.0, "whole words keep their timing");
}
