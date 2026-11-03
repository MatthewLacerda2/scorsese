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
        ["then Luiz Inácio Lula da Silva", "spoke"],
        "a name longer than the bound is a caption of its own, not cut"
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
