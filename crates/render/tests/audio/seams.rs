//! A long sound under a cut on another track.
//!
//! The mix is cut into stretches wherever the audible set changes, so a music
//! bed under a narration line crosses a seam where the line starts and another
//! where it ends. These pin that a compressed bed loses nothing at either: the
//! mix of it is the one uninterrupted decode of the file, sample for sample.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Clip, Fps, ProjectPath};
use scorsese_render::audio::{CHANNELS, mixdown};
use scorsese_render::testing::tone_graph;
use scorsese_render::{FrameRange, Plan, RenderSettings, Resolution, Tools};

use crate::common::audio::{RATE, tone_asset};
use crate::common::ffmpeg::{generate, tools};
use crate::common::{audio_track, project, scratch};

/// Where the line on the other track starts and ends, in seconds: two seams,
/// both well inside the bed.
const SEAMS: [f64; 2] = [1.0, 2.0];

/// What a near-silent sample is, against a bed peaking at a half.
const NEAR_SILENT: f32 = 0.01;

#[test]
fn an_mp3_bed_is_continuous_across_the_seams_of_a_line_over_it() {
    bed_is_continuous("mp3", &["-c:a", "libmp3lame", "-b:a", "128k"]);
}

#[test]
fn an_m4a_bed_is_continuous_across_the_seams_of_a_line_over_it() {
    bed_is_continuous("m4a", &["-c:a", "aac", "-b:a", "128k"]);
}

/// Renders the mix of a four-second bed of `extension` under a silent line
/// from one second to two, and holds it against one decode of the whole file.
fn bed_is_continuous(extension: &str, codec: &[&str]) {
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

    let mixed = mix_of(&tools, &project, &dir);
    let whole = one_decode(&tools, &dir.join(format!("assets/bed.{extension}")));
    assert!(
        mixed.len() <= whole.len(),
        "the mix ({} samples) outran the file ({})",
        mixed.len(),
        whole.len()
    );

    for seam in SEAMS {
        let at = (seam * f64::from(RATE)) as usize * CHANNELS;
        let after = &mixed[at..at + RATE as usize / 20 * CHANNELS];
        let quiet = after.iter().filter(|s| s.abs() < NEAR_SILENT).count();
        let expected = whole[at..at + after.len()]
            .iter()
            .filter(|s| s.abs() < NEAR_SILENT)
            .count();
        assert!(
            quiet <= expected,
            "{extension}: {quiet} near-silent samples in the 50 ms after the seam at {seam} s, \
             where one decode has {expected}"
        );
    }
    let worst = mixed
        .iter()
        .zip(&whole)
        .enumerate()
        .map(|(i, (a, b))| (i, (a - b).abs()))
        .fold((0, 0.0_f32), |w, d| if d.1 > w.1 { d } else { w });
    assert!(
        worst.1 < 1e-5,
        "{extension}: the mix departs from one decode by {} at {:.4} s",
        worst.1,
        (worst.0 / CHANNELS) as f64 / f64::from(RATE)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A four-second sine at a half, at 44.1 kHz as Pixabay's music is, encoded
/// with `codec` into `assets/bed.{extension}`.
fn compressed_bed(tools: &Tools, root: &Path, extension: &str, codec: &[&str]) -> Asset {
    let graph = format!("{},aresample=44100", tone_graph(440, "0.5", 4.0));
    let path = format!("assets/bed.{extension}");
    let mut args = vec!["-f", "lavfi", "-i", &graph, "-ac", "2"];
    args.extend_from_slice(codec);
    generate(tools, &root.join(&path), &args);
    Asset::imported(
        AssetId::new("bed"),
        AssetKind::Audio,
        ProjectPath::new(path),
    )
}

/// The project's mix, as the render writes it before encoding.
fn mix_of(tools: &Tools, project: &scorsese_core::Project, root: &Path) -> Vec<f32> {
    let settings = RenderSettings::new(
        Resolution::new(32, 32).expect("a legal raster"),
        Fps::THIRTY,
    );
    let plan = Plan::build_sound(project, Fps::THIRTY, FrameRange::ALL).expect("a plan");
    let (mixed, _, _) = mixdown(tools, &settings, &plan, root, &root.join("out.wav"))
        .expect("the mix succeeds")
        .expect("the project has sound");
    floats(&std::fs::read(mixed.path()).expect("read the mix"))
}

/// The whole file decoded once, the way the mix decodes a source.
fn one_decode(tools: &Tools, file: &Path) -> Vec<f32> {
    let output = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(file)
        .args(["-af", "aresample=async=1:first_pts=0", "-vn"])
        .args(["-ar", &RATE.to_string(), "-ac", &CHANNELS.to_string()])
        .args(["-f", "f32le", "-"])
        .output()
        .expect("run ffmpeg");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    floats(&output.stdout)
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(size_of::<f32>())
        .map(|word| f32::from_le_bytes(word.try_into().expect("four bytes")))
        .collect()
}
