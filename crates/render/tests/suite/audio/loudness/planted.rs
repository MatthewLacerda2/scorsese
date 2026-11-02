//! A link waiting beside the output at a name the mix or its rehearsal used
//! to be written under is never written through (#1672).

#![cfg(unix)]

use std::path::PathBuf;

use scorsese_core::{Clip, Fps};
use scorsese_render::audio::measure;
use scorsese_render::{Container, FrameRange, OutputFormat, RenderSettings, Renderer, Resolution};

use super::headroom::{LEVEL, tone};
use crate::audio::picture;
use crate::common::ffmpeg::tools;
use crate::common::scratch;
use crate::common::{audio_track, project, video_track};

/// A link waiting beside the output at a name the mix, its rehearsal or the
/// encode was written under on main, or at the folders they are written in
/// now, is left as it was, and the delivery still lands (#1672).
#[test]
fn a_link_beside_the_output_is_never_written_through() {
    let tools = tools();
    let dir = scratch::dir("headroom-planted");
    let outside = scratch::dir("headroom-planted-outside");
    let victim = outside.join("victim.txt");
    std::fs::write(&victim, "the victim's own bytes").unwrap();
    let project = project(
        vec![picture(&tools, &dir, 2), tone(&tools, &dir, true, LEVEL)],
        vec![
            video_track("v1", vec![Clip::at("c1", "picture", 0, 60)]),
            audio_track("a1", vec![Clip::at("m1", "tone", 0, 60)]),
        ],
    );
    let out = dir.join("out.mp4");
    let mut planted: Vec<PathBuf> = [
        "scorsese-mix.pcm",
        "scorsese-rehearsal",
        "scorsese-unraised.pcm",
        "partial",
    ]
    .iter()
    .map(|tag| dir.join(format!(".out.mp4.{tag}")))
    .collect();
    for link in &planted {
        std::os::unix::fs::symlink(&victim, link).unwrap();
    }
    for tag in ["scorsese-audio", "rendering"] {
        let link = dir.join(format!(".out.mp4.{tag}"));
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        planted.push(link);
    }
    let before: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    let settings = RenderSettings::new(Resolution::new(32, 32).expect("a raster"), Fps::THIRTY)
        .with_format(OutputFormat::defaults_for(Container::Mp4));

    Renderer::new(&tools, settings)
        .render(&project, &dir, FrameRange::ALL, &out)
        .expect("the render succeeds");

    assert!(
        std::fs::read(&victim).unwrap() == b"the victim's own bytes",
        "the link's target was written through"
    );
    assert_eq!(
        std::fs::read_dir(&outside).unwrap().count(),
        1,
        "nothing was written through a linked folder"
    );
    assert!(measure(&tools, &out).is_ok(), "the delivery still lands");
    let left: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path != &out && !before.contains(path))
        .collect();
    assert!(left.is_empty(), "the render left scratch behind: {left:?}");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&outside).ok();
}
