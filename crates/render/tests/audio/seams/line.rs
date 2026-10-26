//! A bed under a line on another track, crossing the seams where it starts
//! and ends.

use scorsese_core::Clip;

use super::{M4A, MP3, SEAMS, compressed_bed, holds_to_one_decode};
use crate::common::audio::tone_asset;
use crate::common::ffmpeg::tools;
use crate::common::{audio_track, project, scratch};

#[test]
fn an_mp3_bed_is_continuous_across_the_seams_of_a_line_over_it() {
    under_a_line("mp3", MP3);
}

#[test]
fn an_m4a_bed_is_continuous_across_the_seams_of_a_line_over_it() {
    under_a_line("m4a", M4A);
}

/// Four seconds of a bed of `extension` under a silent line from one second to
/// two.
fn under_a_line(extension: &str, codec: &[&str]) {
    // #1607: every stretch decoded the bed again with an input seek, which in
    // a compressed sound does not land sample-exact — an mp3 opened ~31 ms
    // of near-silence at each seam, an .m4a dropped its first frame.
    let tools = tools();
    let dir = scratch::dir(&format!("seams-{extension}"));
    let bed = compressed_bed(&tools, &dir, extension, codec);
    let line = tone_asset(&tools, &dir, "line", 440, 1.0, 0.0, "");
    let project = project(
        vec![bed, line],
        vec![
            audio_track("music", vec![Clip::at("bed", "bed", 0, 120)]),
            audio_track("voice", vec![Clip::at("line", "line", 30, 30)]),
        ],
    );
    holds_to_one_decode(&tools, &project, &dir, extension, 0.0, &SEAMS);
}
