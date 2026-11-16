//! A glance's transcript: the words, one line per phrase, each led by when
//! it starts.

use scorsese_core::words::Words;

use super::super::sample::{self, label};

/// The most words one line of a transcript holds before it is broken.
const WORDS_PER_LINE: usize = 16;

/// A pause this long, in seconds, between two words starts a new line.
const LINE_PAUSE: f64 = 1.0;

/// `words` as lines, each led by when its first word starts: broken after a
/// sentence ends, at a pause of [`LINE_PAUSE`], or at [`WORDS_PER_LINE`].
/// `None` when there are no words.
pub(super) fn transcript(words: &Words) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line: Vec<&str> = Vec::new();
    let mut started = 0.0;
    let mut previous_end: Option<f64> = None;
    for word in &words.words {
        let paused = previous_end.is_some_and(|end| word.start - end >= LINE_PAUSE);
        if paused && !line.is_empty() {
            lines.push(said_at(started, &line));
            line.clear();
        }
        if line.is_empty() {
            started = word.start;
        }
        line.push(&word.text);
        previous_end = Some(word.end);
        let ended = word.text.ends_with(['.', '?', '!', '…']);
        if ended || line.len() >= WORDS_PER_LINE {
            lines.push(said_at(started, &line));
            line.clear();
        }
    }
    if !line.is_empty() {
        lines.push(said_at(started, &line));
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// One line of a transcript: when it starts, as a cell's label reads, then
/// its words.
fn said_at(seconds: f64, words: &[&str]) -> String {
    format!("{} {}", label(sample::tenths(seconds)), words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transcript_breaks_at_sentences_and_pauses() {
        let word = |text: &str, start: f64| scorsese_core::words::Word {
            text: text.to_owned(),
            start,
            end: start + 0.3,
        };
        let words = Words {
            words: vec![
                word("So", 0.0),
                word("then", 0.4),
                word("later", 5.0),
                word("on.", 5.4),
                word("Done", 6.0),
            ],
        };
        assert_eq!(
            transcript(&words).as_deref(),
            Some("0:00 So then\n0:05 later on.\n0:06 Done")
        );
    }
}
