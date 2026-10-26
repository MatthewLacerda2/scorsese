//! A bed opening partway into its file: where the first decode of it seeks.

use scorsese_core::Clip;

use super::{M4A, MP3, compressed_bed, holds_to_one_decode};
use crate::common::ffmpeg::tools;
use crate::common::{audio_track, clip_from, project, scratch};

#[test]
fn an_mp3_bed_split_in_two_is_continuous_across_the_split() {
    split_in_two("mp3", MP3);
}

#[test]
fn an_m4a_bed_split_in_two_is_continuous_across_the_split() {
    split_in_two("m4a", M4A);
}

#[test]
fn an_mp3_bed_trimmed_at_its_head_opens_without_a_hole() {
    trimmed_at_its_head("mp3", MP3);
}

#[test]
fn an_m4a_bed_trimmed_at_its_head_opens_without_a_hole() {
    trimmed_at_its_head("m4a", M4A);
}

/// The bed as two back-to-back clips, the second opening where the first
/// stops: two seconds into the file, two seconds into the mix.
fn split_in_two(extension: &str, codec: &[&str]) {
    // #1607: a clip's first decode seeked straight to where it enters its
    // file, so the second half opened on the same hole a seam used to cost.
    let tools = tools();
    let dir = scratch::dir(&format!("split-{extension}"));
    let bed = compressed_bed(&tools, &dir, extension, codec);
    let project = project(
        vec![bed],
        vec![audio_track(
            "music",
            vec![
                Clip::at("first", "bed", 0, 60),
                clip_from("second", "bed", 60, 60, 60),
            ],
        )],
    );
    holds_to_one_decode(&tools, &project, &dir, extension, 0.0, &[2.0]);
}

/// The bed entering its file a second in, at the top of the mix.
fn trimmed_at_its_head(extension: &str, codec: &[&str]) {
    let tools = tools();
    let dir = scratch::dir(&format!("head-trim-{extension}"));
    let bed = compressed_bed(&tools, &dir, extension, codec);
    let project = project(
        vec![bed],
        vec![audio_track(
            "music",
            vec![clip_from("bed", "bed", 0, 90, 30)],
        )],
    );
    holds_to_one_decode(&tools, &project, &dir, extension, 1.0, &[0.0]);
}
