//! What is said beside the sheet: the transcript a glance is handed.

use scorsese_core::words::{Word, Words};
use scorsese_render::contact;

use super::changing;
use crate::common::ffmpeg::tools;
use crate::common::scratch;

fn word(text: &str, start: f64, end: f64) -> Word {
    Word {
        text: text.to_owned(),
        start,
        end,
    }
}

/// The transcript comes beside the sheet, each line led by when it is said
/// in the same seconds the frames are labelled with; none comes when there
/// are no words.
#[test]
fn the_transcript_rides_beside_the_sheet_in_the_sheets_seconds() {
    let dir = scratch::dir("glance-words");
    let tools = tools();
    let file = changing(&tools, &dir, 0.0, 20.0);
    let words = Words {
        words: vec![
            word("Welcome", 1.2, 1.6),
            word("back.", 1.7, 2.0),
            word("Today", 12.0, 12.4),
            word("we", 12.5, 12.6),
            word("cook.", 12.7, 13.0),
        ],
    };
    let glance = contact::glance(&tools, &file, Some(&words), false).expect("a glance");
    assert_eq!(
        glance.transcript.as_deref(),
        Some("0:01.2 Welcome back.\n0:12 Today we cook.")
    );
    let silent = contact::glance(&tools, &file, Some(&Words::default()), false).expect("a glance");
    assert_eq!(silent.transcript, None, "no words is no transcript");
}
