//! The invariant: a script and a note never reach the screen.
//!
//! This is worth a test of its own rather than trust, because the format has
//! precedent for text in a document being drawn — a `text` asset is glyphs, and
//! a sketch asset's `prompt` goes on a slug card — so "text in `project.json`
//! that is never rendered" is the exception here, not the rule. A future change
//! that started drawing one would look, from inside, exactly like the changes
//! that draw the others.
//!
//! So it is checked the only way that proves anything: annotate every element
//! the format allows, composite real frames, strip every annotation, composite
//! again, and compare **byte for byte**. No tolerance — this is not a question
//! about an encoder, it is a question about whether the compositor ever saw the
//! text at all.

mod common;

use scorsese_core::{AssetKind, Fps, Frames, Project, ProjectPath};
use scorsese_render::{Frame, RenderSettings, Renderer, Resolution};

use common::ffmpeg::{fixture_dir, generate, tools};
use common::{clip, file_asset, sketch_asset, text_asset, video_track};

/// Every frame the fixture is compared on: a shot, a title over it, and the
/// slug card of a prompt nobody has generated — the three ways something
/// reaches the picture, one of which draws a document's own words.
const FRAMES: [u64; 3] = [5, 35, 65];

/// A cut with one of each, and a real file behind the shot.
fn film(dir: &std::path::Path, tools: &scorsese_render::Tools) -> Project {
    generate(
        tools,
        &dir.join("assets").join("shot.mp4"),
        &["-f", "lavfi", "-i", "testsrc=size=64x64:duration=1:rate=30"],
    );
    Project {
        assets: vec![
            file_asset("shot", AssetKind::Video),
            text_asset("title"),
            sketch_asset("dream"),
        ],
        tracks: vec![video_track(
            "v1",
            vec![
                clip("c-shot", "shot", 0, 30),
                clip("c-title", "title", 30, 30),
                clip("c-dream", "dream", 60, 30),
            ],
        )],
        ..Project::new("Annotated", Fps::THIRTY)
    }
}

/// The same cut, carrying a script and a note on every element that can hold
/// one — deliberately worded like something an editor would want on screen, so
/// a leak would be unmistakable rather than a stray pixel.
fn annotated(mut project: Project) -> Project {
    project.script = Some(ProjectPath::new("script.md"));
    for asset in &mut project.assets {
        asset.note = Some(format!("NOTE ON ASSET {}", asset.id));
    }
    for track in &mut project.tracks {
        track.note = Some(format!("NOTE ON TRACK {}", track.id));
        for clip in &mut track.clips {
            clip.note = Some(format!("NOTE ON CLIP {}", clip.id));
        }
    }
    project
}

fn stills(dir: &std::path::Path, project: &Project) -> Vec<Frame> {
    let tools = tools();
    let settings = RenderSettings::new(
        Resolution::new(64, 64).expect("a legal raster"),
        Fps::THIRTY,
    );
    let renderer = Renderer::new(&tools, settings);
    FRAMES
        .iter()
        .map(|&at| {
            renderer
                .still(project, dir, Frames(at))
                .unwrap_or_else(|error| panic!("compositing frame {at}: {error}"))
        })
        .collect()
}

#[test]
fn stripping_every_note_and_the_script_renders_identical_frames() {
    let dir = fixture_dir("annotations");
    let tools = tools();
    let plain = film(&dir, &tools);
    // The script file is written, so this is not passing merely because a
    // missing file was never opened.
    std::fs::write(dir.join("script.md"), "NEVER ON SCREEN\n").expect("write the script");

    let before = stills(&dir, &annotated(plain.clone()));
    let after = stills(&dir, &plain);

    for (index, at) in FRAMES.iter().enumerate() {
        assert_eq!(
            before[index].bytes(),
            after[index].bytes(),
            "frame {at} differs; a note or the script reached the picture"
        );
    }
}
