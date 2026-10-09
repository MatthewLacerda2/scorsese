//! A narration line not yet generated, drawn and left out (#966): the band is
//! pixels over the foot of the shot, and leaving it out leaves the shot alone.

use scorsese_core::{Fps, Frames, Project};
use scorsese_render::{Bands, Frame, FrameRange, Note, Renderer};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, narration_asset, project, video_track};
use crate::{RASTER, RED, assert_colour, colour_asset, settings};

/// A pixel in the bottom row, at the left edge where the band's text never
/// reaches.
fn foot(frame: &Frame) -> (u8, u8, u8) {
    let at = ((RASTER.1 - 1) * RASTER.0 * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

fn narrated(dir: &std::path::Path) -> Project {
    project(
        vec![
            colour_asset(&tools(), dir, "red", "64x64", 1),
            narration_asset("vo"),
        ],
        vec![
            video_track("v1", vec![clip("shot", "red", 0, 20)]),
            audio_track("a1", vec![clip("vo1", "vo", 0, 20)]),
        ],
    )
}

#[test]
fn a_band_is_drawn_unless_the_render_leaves_it_out() {
    let tools = tools();
    let dir = fixture_dir("narration-bands");
    let project = narrated(&dir);
    let at = Frames(10);

    let drawn = Renderer::new(&tools, settings(Fps::THIRTY))
        .still(&project, &dir, at)
        .expect("a still");
    let (red, ..) = foot(&drawn);
    assert!(red < 120, "the band darkens the foot of the shot: {red}");

    let omitted = Renderer::new(&tools, settings(Fps::THIRTY).with_bands(Bands::Omitted));
    let frame = omitted.still(&project, &dir, at).expect("a still");
    assert_colour(foot(&frame), RED, "the shot, with nothing over it");

    let out = dir.join("out.mp4");
    let report = omitted
        .render(&project, &dir, FrameRange::ALL, &out)
        .expect("the render succeeds");
    assert!(
        report.notes.contains(&Note::BandsLeftOut {
            clips: vec!["vo1".to_owned()]
        }),
        "the report says which lines it left out: {:?}",
        report.notes
    );
    std::fs::remove_dir_all(&dir).ok();
}
