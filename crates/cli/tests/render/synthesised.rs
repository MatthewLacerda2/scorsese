//! Synthesised sound, all the way to a delivered file.
//!
//! Every other synth test stops at `generated/`. This one is the whole point
//! of the feature: a recipe an agent wrote becomes a soundtrack, with no media
//! supplied by anyone and nothing bought from anybody.

use std::path::PathBuf;

use crate::common::documents::{self, assets};
use crate::common::media::probe::probe;
use crate::common::media::tools;
use crate::common::{media, run_in, temp_dir};
use crate::render;

/// A project whose only sound is a baked recipe, under a plain colour shot.
///
/// The synth asset's document entry is copied out of the project `synth bake`
/// left behind rather than hand-written, so the test cannot assert against a
/// row the real command would never produce.
fn scored(label: &str) -> PathBuf {
    let dir = temp_dir(label);
    media::colour_video(&tools(), &dir, "red", "red", 4);
    documents::write_into(&dir, &[assets::raw("red", "video", "mp4")], &[]);

    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    run_in(&dir, &["synth", "bake"]).ok();

    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("project.json")).expect("read"))
            .expect("the baked document parses");
    let theme = document["assets"]
        .as_array()
        .expect("an assets table")
        .iter()
        .find(|asset| asset["id"] == "theme")
        .expect("the baked asset")
        .to_string();

    documents::write_into(
        &dir,
        &[assets::raw("red", "video", "mp4"), theme],
        &[
            documents::video_track(&[documents::lasting("c1", "red", 0, 60)]),
            documents::audio_track(&[documents::lasting("a1c", "theme", 0, 60)]),
        ],
    );
    dir
}

#[test]
fn a_recipe_becomes_the_soundtrack_of_a_delivered_file() {
    let dir = scored("synth-render");
    let rendered = render(&dir, "cut.mp4", &[]);
    let file = rendered.file();

    // The mix is stereo at the render's rate, though the bake was mono at
    // 44.1 kHz: a synthesised file takes the same conforming path into the mix
    // that an imported one does, which is why synthesis never had to know what
    // a render was going to be asked for.
    let stream = probe(
        &tools(),
        &file,
        "a:0",
        "stream=sample_rate,channels,codec_name",
    );
    assert_eq!(stream.number("sample_rate"), 48_000);
    assert_eq!(stream.number("channels"), 2);
    assert_eq!(stream.get("codec_name"), "aac");
    std::fs::remove_dir_all(&dir).ok();
}

/// A synth asset is not a stand-in: once baked it is media, so the render must
/// not report it as one, and must not stand silence in for it.
#[test]
fn the_render_treats_it_as_media_rather_than_a_card() {
    let dir = scored("synth-render-notes");
    let rendered = render(&dir, "cut.mp4", &[]);
    let run = rendered.run.ok();
    run.silent_about("stood in");
    run.silent_about("card");
    std::fs::remove_dir_all(&dir).ok();
}
