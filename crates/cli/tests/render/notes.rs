//! The report's warnings, which are how an unattended run learns it went
//! wrong.
//!
//! Every one of these describes a render that **succeeded** and may not have
//! produced what its author meant. So each test asserts both halves: the note
//! reached the caller, and the file was delivered anyway. A note that stopped
//! a render, or a render that quietly dropped a track, are the two failures
//! this whole mechanism exists to prevent — and only asserting both catches
//! either.

use std::path::PathBuf;

use crate::common::documents::{self, assets};
use crate::common::media::probe::probe;
use crate::common::media::tools;
use crate::common::{media, temp_dir};
use crate::{Rendered, one_shot, render};

/// How many frames the render actually wrote.
#[track_caller]
fn frames(rendered: Rendered) -> u64 {
    probe(&tools(), &rendered.file(), "v:0", "stream=nb_read_frames").number("nb_read_frames")
}

#[test]
fn a_range_running_past_the_end_of_the_timeline_is_reported_and_clamped() {
    let dir = one_shot("clamped", 30);
    let rendered = render(&dir, "cut.mp4", &["--range", "0:90"]);

    rendered.run.says("the range asked for frame 90");
    rendered.run.says("the timeline ends at 30");
    assert_eq!(frames(rendered), 30, "clamped, not padded");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_clip_outlasting_its_own_source_is_reported_and_rendered_black() {
    // One second of media under a two-second clip. The film is still two
    // seconds long — the shot just runs out, which is the thing worth saying.
    let dir = temp_dir("short-source");
    media::colour_video(&tools(), &dir, "red", "red", 1);
    documents::write_into(
        &dir,
        &[assets::raw("red", "video", "mp4")],
        &[documents::video_track(&[documents::lasting(
            "c1", "red", 0, 60,
        )])],
    );

    let rendered = render(&dir, "cut.mp4", &[]);
    rendered
        .run
        .says("clip `c1` outlasts its source by 30 frame(s)");
    rendered.run.says("rendered black");
    assert_eq!(frames(rendered), 60);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_generated_shot_with_no_file_says_a_slug_card_stood_in_for_it() {
    // The state a deleted cache leaves behind: the project believes the money
    // was spent, and the file is not there. A preview cut with one card in it
    // beats no preview at all — but only if the report says which card.
    let dir = documents::written(
        "generated-gone",
        &[assets::generated("boat", "video", "mp4")],
        &[documents::video_track(&[documents::lasting(
            "c1", "boat", 0, 15,
        )])],
    );

    let rendered = render(&dir, "cut.mp4", &[]);
    rendered.run.says("clip `c1` shows asset `boat`");
    rendered.run.says("has no file on disk");
    rendered.run.says("a slug card stood in for it");
    assert_eq!(frames(rendered), 15);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_generated_narration_with_no_file_says_silence_stood_in_for_it() {
    // The other half of the same fault, and not the same finding: a card is
    // visible to whoever watches the result, silence is not.
    let dir = temp_dir("narration-gone");
    media::colour_video(&tools(), &dir, "red", "red", 1);
    documents::write_into(
        &dir,
        &[
            assets::raw("red", "video", "mp4"),
            assets::generated("voice", "audio", "mp3"),
        ],
        &[
            documents::video_track(&[documents::lasting("c1", "red", 0, 15)]),
            documents::audio_track(&[documents::lasting("a1c", "voice", 0, 15)]),
        ],
    );

    let rendered = render(&dir, "cut.mp4", &[]);
    rendered.run.says("clip `a1c` shows asset `voice`");
    rendered.run.says("silence stood in for it");
    assert_eq!(frames(rendered), 15);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_soundtrack_outlasting_the_last_picture_is_reported_as_trimmed() {
    // Picture decides how long the film is. Audio lost that way is reported,
    // never dropped in silence — this is the report that makes that true.
    let dir = temp_dir("trimmed");
    let tools = tools();
    media::colour_video(&tools, &dir, "red", "red", 4);
    media::tone(&tools, &dir, "bed", 4);
    documents::write_into(
        &dir,
        &[
            assets::raw("red", "video", "mp4"),
            assets::raw("bed", "audio", "wav"),
        ],
        &[
            documents::video_track(&[documents::lasting("c1", "red", 0, 15)]),
            documents::audio_track(&[documents::lasting("a1c", "bed", 0, 90)]),
        ],
    );

    let rendered = render(&dir, "cut.mp4", &[]);
    rendered.run.says("audio runs to frame 90");
    rendered.run.says("the last picture ends at 15");
    assert_eq!(frames(rendered), 15, "the picture decides the length");
    std::fs::remove_dir_all(&dir).ok();
}

/// Nothing to say is itself an answer: a healthy render must not invent notes,
/// or the ones above stop meaning anything.
#[test]
fn a_render_with_nothing_wrong_with_it_says_nothing() {
    let dir = one_shot("quiet", 15);
    let rendered = render(&dir, "cut.mp4", &[]);

    rendered.run.silent_about("note:");
    let _: PathBuf = rendered.file();
    std::fs::remove_dir_all(&dir).ok();
}
