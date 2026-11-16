//! A recording's transcript, found by the recording's own path (#1563).

use crate::words::{Word, Words};

/// A file's transcript is found by the file's own path, project or not —
/// what `look` reads beside a video it is pointed at (#1563) — and a file
/// with none, or one that will not read, has none.
#[test]
fn a_recordings_transcript_is_read_beside_it_by_its_path() {
    let dir = crate::scratch::fresh("scorsese-words-heard-in", "");
    std::fs::create_dir_all(&dir).unwrap();
    let film = dir.join("take.mp4");
    assert_eq!(Words::heard_in(&film), None);
    let heard = Words {
        words: vec![Word {
            text: "Hi.".into(),
            start: 1.0,
            end: 1.25,
        }],
    };
    std::fs::write(dir.join("take.mp4.words.json"), heard.to_json()).unwrap();
    assert_eq!(Words::heard_in(&film), Some(heard));
    std::fs::write(dir.join("take.mp4.words.json"), "not json").unwrap();
    assert_eq!(Words::heard_in(&film), None);
}
