//! The page look's cut (#1550): a few words at a time, a name kept whole.

use super::{said, texts};
use crate::ClipId;
use crate::captions::{Chunking, chunks};

#[test]
fn a_caption_holds_at_most_three_words() {
    assert_eq!(
        texts("we looked at every single report today", Chunking::WORDS),
        ["we looked at", "every single report", "today"]
    );
}

#[test]
fn a_comma_breaks_as_well_as_a_sentence_end() {
    assert_eq!(
        texts("Yes, it works. Again", Chunking::WORDS),
        ["Yes,", "it works.", "Again"]
    );
}

#[test]
fn a_half_second_pause_breaks() {
    let mut words = said("one two three");
    words[2].start += 0.5;
    words[2].end += 0.5;
    let cut: Vec<String> = chunks(&ClipId::new("vo"), &words, Chunking::WORDS)
        .into_iter()
        .map(|chunk| chunk.text)
        .collect();
    assert_eq!(cut, ["one two", "three"]);
}

/// The DataForce Story's line: "presidente" and "Lula" landed in two
/// captions, and were asked for together.
#[test]
fn a_name_and_the_word_before_it_are_never_split() {
    assert_eq!(
        texts("disse hoje o presidente Lula em Brasília", Chunking::WORDS),
        ["disse hoje o", "presidente Lula", "em Brasília"]
    );
    assert_eq!(
        texts("then Luiz Inácio Lula da Silva spoke", Chunking::WORDS),
        ["then", "Luiz Inácio Lula", "da Silva spoke"],
        "a name is never longer than the bound"
    );
}

#[test]
fn the_pronoun_i_and_a_sentence_start_are_not_names() {
    assert_eq!(
        texts("so I think we should go", Chunking::WORDS),
        ["so I think", "we should go"]
    );
    assert_eq!(
        texts("It rained. Then it stopped", Chunking::WORDS),
        ["It rained.", "Then it stopped"]
    );
}

#[test]
fn the_text_look_keeps_its_cut() {
    assert_eq!(
        texts(
            "disse hoje o presidente Lula em Brasília",
            Chunking::DEFAULT
        ),
        ["disse hoje o presidente Lula em", "Brasília"]
    );
}

#[test]
fn a_caption_keeps_its_words_named_as_a_page_looks_them_up() {
    let cut = chunks(&ClipId::new("vo"), &said("Hello there"), Chunking::WORDS);
    let names: Vec<&str> = cut[0].words.iter().map(|word| word.name.as_str()).collect();
    assert_eq!(names, ["hello", "there"]);
}

#[test]
fn a_particle_keeps_a_surname_with_its_name() {
    assert_eq!(
        texts("o presidente Lula da Silva falou", Chunking::WORDS),
        ["o presidente", "Lula da Silva", "falou"]
    );
}

#[test]
fn i_in_any_contraction_is_not_a_name() {
    for i in ["I", "I'm", "I’ll"] {
        let line = format!("we all know {i} right");
        assert_eq!(
            texts(&line, Chunking::WORDS),
            ["we all know", &format!("{i} right")]
        );
    }
}

#[test]
fn a_pause_inside_a_name_still_breaks_it() {
    let word = |text: &str, start, end| crate::words::Placed {
        name: text.to_lowercase(),
        text: text.to_owned(),
        start,
        end,
    };
    let words = [word("presidente", 0.5, 1.0), word("Lula", 1.5, 1.8)];
    let cut: Vec<String> = chunks(&ClipId::new("vo"), &words, Chunking::WORDS)
        .into_iter()
        .map(|chunk| chunk.text)
        .collect();
    assert_eq!(cut, ["presidente", "Lula"], "a pause of exactly the bound");
}

/// A particle joins only between two capitalised words, and nothing between
/// them breaks.
#[test]
fn a_particle_outside_a_name_is_an_ordinary_word() {
    assert_eq!(
        texts("o presidente Lula da casa falou", Chunking::WORDS),
        ["o presidente Lula", "da casa falou"]
    );
    assert_eq!(
        texts("ele falou da Bahia hoje", Chunking::WORDS),
        ["ele falou", "da Bahia hoje"]
    );
}

/// Title Case and capitals are not one long name (#1735's review).
#[test]
fn a_line_in_capitals_or_title_case_is_cut_as_any_other() {
    assert_eq!(
        texts("THIS IS THE BEST DEAL EVER.", Chunking::WORDS),
        ["THIS IS THE", "BEST DEAL EVER."]
    );
    assert_eq!(
        texts("Watch How We Built This House", Chunking::WORDS),
        ["Watch How We", "Built This House"]
    );
}

#[test]
fn the_pronoun_with_punctuation_is_still_the_pronoun() {
    assert_eq!(
        texts("my friend and I. Tired", Chunking::WORDS),
        ["my friend and", "I.", "Tired"]
    );
    assert_eq!(
        texts("we went to Plan B", Chunking::WORDS),
        ["we went", "to Plan B"]
    );
}
