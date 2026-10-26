//! Reading a transcript and correcting a misheard word (#1549).

use crate::words::{Fix, FixError, Word, Words};

fn heard() -> Words {
    let said = |text: &str, start: f64, end: f64| Word {
        text: text.into(),
        start,
        end,
    };
    Words {
        words: vec![
            said("Olha", 0.0, 0.32),
            said("o", 0.32, 0.4),
            said("casa", 0.4, 0.71),
            said("bê", 0.71, 0.9),
            said("aqui.", 0.9, 1.25),
        ],
    }
}

fn fix(word: usize, text: &str) -> Fix {
    Fix {
        word,
        text: text.into(),
    }
}

#[test]
fn a_listing_numbers_each_word_from_one_with_its_times() {
    assert_eq!(
        heard().listing(),
        "   1  0.00–0.32  Olha\n   2  0.32–0.40  o\n   3  0.40–0.71  casa\n   \
         4  0.71–0.90  bê\n   5  0.90–1.25  aqui."
    );
    assert_eq!(Words::default().listing(), "");
}

#[test]
fn a_fixed_word_keeps_its_times_and_an_empty_one_is_dropped() {
    let mut words = heard();
    let dropped = words
        .fix(&[fix(3, "Kassab"), fix(4, ""), fix(1, " Olha, ")])
        .unwrap();
    assert_eq!(dropped, 1);
    let read: Vec<(&str, f64, f64)> = words
        .words
        .iter()
        .map(|w| (w.text.as_str(), w.start, w.end))
        .collect();
    assert_eq!(
        read,
        [
            ("Olha,", 0.0, 0.32),
            ("o", 0.32, 0.4),
            ("Kassab", 0.4, 0.71),
            ("aqui.", 0.9, 1.25)
        ],
        "numbers are the listing's before the fix, so a drop renumbers nothing"
    );
}

#[test]
fn a_fix_naming_no_word_or_one_word_twice_changes_nothing() {
    let mut words = heard();
    assert_eq!(
        words.fix(&[fix(2, "a"), fix(6, "x")]),
        Err(FixError::NoSuchWord { word: 6, count: 5 })
    );
    assert_eq!(
        words.fix(&[fix(0, "x")]),
        Err(FixError::NoSuchWord { word: 0, count: 5 })
    );
    assert_eq!(
        words.fix(&[fix(2, "a"), fix(2, "e")]),
        Err(FixError::Twice(2))
    );
    assert_eq!(words, heard());
    assert_eq!(
        FixError::NoSuchWord { word: 6, count: 5 }.to_string(),
        "there is no word 6: the transcript has 5, numbered from 1"
    );
}
