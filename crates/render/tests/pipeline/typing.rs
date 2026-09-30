//! Text that is set afresh every frame: a reveal and a counter, through the
//! whole pipeline and its workers.

use scorsese_core::{
    Asset, AssetId, Counter, Easing, Fps, Frames, Keyframe, KeyframeTrack, PropertyPath, Rgba,
    TextStyle,
};
use scorsese_render::{Frame, FrameRange, Renderer};

use crate::common::ffmpeg::{fixture_dir, mean_rgb, tools};
use crate::common::{clip, project, video_track};
use crate::{render, settings};

fn caption(content: &str, number: Option<Counter>) -> Asset {
    Asset {
        style: Some(TextStyle {
            color: Rgba::WHITE,
            size: 0.3,
            number,
            ..TextStyle::default()
        }),
        ..Asset::text(AssetId::new("caption"), content)
    }
}

fn ramp(property: &str, to: f64) -> KeyframeTrack {
    let key = |t: u64, value: f64| Keyframe {
        t: Frames(t),
        value,
        easing: Easing::Linear,
    };
    KeyframeTrack::new(PropertyPath::new(property), vec![key(0, 0.0), key(10, to)])
}

/// How much ink is on a frame still in memory.
fn lit(frame: &Frame) -> u64 {
    frame
        .bytes()
        .chunks_exact(4)
        .map(|pixel| u64::from(pixel[0]))
        .sum()
}

/// A reveal is drawn by the workers, frame by frame: nothing at its start, all
/// of it at its end — and the file agrees with a still at the same instant,
/// so the per-frame drawing is not something only a preview does.
#[test]
fn a_revealing_caption_is_set_again_for_every_frame() {
    let tools = tools();
    let dir = fixture_dir("typing-reveal");
    let mut revealing = clip("c1", "caption", 0, 20);
    revealing.keyframes.push(ramp("reveal", 1.0));
    let project = project(
        vec![caption("Ship it now", None)],
        vec![video_track("v1", vec![revealing])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);
    assert_eq!(report.frames, 20);
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY));
    let still = |at: u64| {
        renderer
            .still(&project, &dir, Frames(at))
            .expect("inside the edit")
    };
    assert_eq!(lit(&still(0)), 0, "nothing has arrived at reveal 0");
    assert!(lit(&still(5)) > 0 && lit(&still(5)) < lit(&still(15)));
    assert_eq!(
        mean_rgb(&tools, &out, 0),
        (0, 0, 0),
        "the file starts dark too"
    );
    assert!(
        mean_rgb(&tools, &out, 15).0 > 2,
        "and has the caption by the end"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A counter's figure is written per frame: two instants of the same clip are
/// two different pictures, and the one with no track holds a single picture.
#[test]
fn a_counting_figure_changes_between_frames() {
    let tools = tools();
    let dir = fixture_dir("typing-count");
    let mut counting = clip("c1", "caption", 0, 20);
    counting.keyframes.push(ramp("number", 1000.0));
    let project = project(
        vec![caption("{n}", Some(Counter::default()))],
        vec![video_track("v1", vec![counting])],
    );
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY));
    let at = |frame: u64| {
        renderer
            .still(&project, &dir, Frames(frame))
            .expect("inside the edit")
    };
    assert!(at(1).bytes() != at(12).bytes(), "100 and 1,000 differ");
    assert!(
        at(12).bytes() == at(18).bytes(),
        "and the count holds once it lands"
    );
    std::fs::remove_dir_all(&dir).ok();
}
