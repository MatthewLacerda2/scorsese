//! What reads a repeating layer without playing it.

use scorsese_zimmer::Song;
use scorsese_zimmer::midi::export;
use scorsese_zimmer::song::InlineOnly;
use scorsese_zimmer::survey::SongSurvey;

use super::{written_once, written_out};

/// Every walker sees the passes, not just the renderer: the survey counts
/// them and the MIDI file writes them.
#[test]
fn the_survey_and_the_midi_file_hear_every_pass() {
    let survey = |song: &Song| SongSurvey::of("band", song, &InlineOnly).tracks;
    assert_eq!(survey(&written_once()), survey(&written_out()));
    assert_eq!(survey(&written_once())[0].notes, 8);

    let midi = |song: &Song| export(song, &[]).expect("exports").bytes;
    assert_eq!(midi(&written_once()), midi(&written_out()));
}
