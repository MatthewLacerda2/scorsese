//! Pulling frames out of a finished render, as PNGs somebody can look at.
//!
//! Needs ffmpeg on PATH, like everything else here that touches real media.
//!
//! The question is only ever "is this the frame I asked for?", and the fixture
//! is built so that the answer is visible in the colour: the shot changes at
//! frame 30, so a still one frame either side of it comes back the wrong
//! colour. That is the whole failure mode — an extraction off by one is silent,
//! and produces a picture that looks perfectly fine.

#[path = "../common/mod.rs"]
mod common;

mod boundaries;
mod cues;

use std::path::{Path, PathBuf};

use scorsese_core::{AssetKind, Fps, Project};
use scorsese_render::frames::{self, Still};
use scorsese_render::{
    Description, Frame, FrameRange, RenderSettings, Renderer, Resolution, Tools,
};

use common::ffmpeg::{fixture_dir, generate_asset, tools};
use common::{clip, file_asset, project, video_track};

/// Red for the first second, blue for the second: the cut is at frame 30.
fn cut_at_thirty(tools: &Tools, root: &Path) -> Project {
    for colour in ["red", "blue"] {
        generate_asset(
            tools,
            root,
            colour,
            AssetKind::Video,
            &[
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={colour}:s=32x32:d=2:r=30"),
                "-pix_fmt",
                "yuv420p",
            ],
        );
    }
    project(
        vec![
            file_asset("red", AssetKind::Video),
            file_asset("blue", AssetKind::Video),
        ],
        vec![video_track(
            "v1",
            vec![clip("c1", "red", 0, 30), clip("c2", "blue", 30, 30)],
        )],
    )
}

fn raster() -> Resolution {
    Resolution::new(32, 32).expect("a legal raster")
}

/// Which of red and blue a decoded frame is, by the channel that dominates it.
#[track_caller]
fn colour_of(frame: &Frame) -> &'static str {
    let mean = |offset: usize| -> u64 {
        let channel: Vec<u64> = frame
            .bytes()
            .iter()
            .skip(offset)
            .step_by(4)
            .map(|&value| u64::from(value))
            .collect();
        channel.iter().sum::<u64>() / channel.len() as u64
    };
    let (red, blue) = (mean(0), mean(2));
    assert!(
        red.abs_diff(blue) > 100,
        "the fixture frame is neither red nor blue: r{red} b{blue}"
    );
    if red > blue { "red" } else { "blue" }
}

/// A rendered fixture, and everything a test needs to ask about it.
struct Rendered {
    tools: Tools,
    out: PathBuf,
    stills: PathBuf,
    description: Description,
}

impl Rendered {
    fn new(label: &str) -> Self {
        let tools = tools();
        let dir = fixture_dir(label);
        let project = cut_at_thirty(&tools, &dir);
        let out = dir.join("out.mp4");
        let report = Renderer::new(&tools, RenderSettings::new(raster(), Fps::THIRTY))
            .render(&project, &dir, FrameRange::ALL, &out)
            .expect("the fixture renders");
        Self {
            tools,
            stills: dir.join("stills"),
            out,
            description: report.description,
        }
    }

    fn write(&self, frames: &[u64]) -> Vec<Still> {
        frames::stills(&self.tools, &self.out, frames, raster(), &self.stills)
            .expect("the stills are written")
    }

    #[track_caller]
    fn colour(&self, still: &Still) -> &'static str {
        assert!(
            still.path.is_file(),
            "{} was not written",
            still.path.display()
        );
        let frame =
            frames::read_png(&self.tools, &still.path, raster()).expect("the still reads back");
        colour_of(&frame)
    }
}

impl Drop for Rendered {
    fn drop(&mut self) {
        if let Some(dir) = self.out.parent() {
            std::fs::remove_dir_all(dir).ok();
        }
    }
}
