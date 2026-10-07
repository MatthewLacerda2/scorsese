//! Word timings (#811): kept beside the audio when the provider gives them,
//! and never paid for again to get them when it did not.

use scorsese_core::words::{Word, Words};
use scorsese_providers::credentials::Budget;
use scorsese_providers::speech::{Outcome, generate};

use crate::mock::{Answer, Mock};
use crate::sketched;

fn said() -> Words {
    Words {
        words: vec![Word {
            text: String::from("line."),
            start: 0.2,
            end: 0.6,
        }],
    }
}

#[test]
fn a_timed_line_keeps_its_words_beside_the_audio() {
    let (dir, mut project, id) = sketched("timed", "the line.");
    let provider = Mock::giving(vec![Answer::Timed(b"MP3".to_vec(), said())]);
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let asset = project.asset(&id).expect("the line");
    let audio = asset.path.clone().expect("spoken");
    assert!(Words::beside(&audio).resolve(&dir).is_file());
    assert_eq!(Words::of(asset, &dir), Some(said()));
    std::fs::remove_dir_all(&dir).ok();
}

/// A line spoken before timings were kept stays a cache hit: it has no word
/// timings, and nothing pays for the line again to get them.
#[test]
fn an_untimed_line_is_never_spoken_again_for_its_words() {
    let (dir, mut project, id) = sketched("untimed", "the line.");
    let provider = Mock::giving(vec![
        Answer::Speaks(b"MP3".to_vec()),
        Answer::Timed(b"MP3".to_vec(), said()),
    ]);
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let again = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert!(matches!(again[0].1, Outcome::Cached { .. }), "{again:?}");
    assert_eq!(provider.requests(), 1);
    assert_eq!(Words::of(project.asset(&id).expect("the line"), &dir), None);
    std::fs::remove_dir_all(&dir).ok();
}
