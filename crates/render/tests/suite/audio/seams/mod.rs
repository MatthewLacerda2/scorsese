//! A long sound under a cut on another track.
//!
//! The mix is cut into stretches wherever the audible set changes, so a music
//! bed under a narration line crosses a seam where the line starts and another
//! where it ends. These pin that a compressed bed loses nothing at either: the
//! mix of it is the one uninterrupted decode of the file, sample for sample.
//! So does a bed that opens partway into its file: split in two, or trimmed at
//! its head.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, ProjectPath};
use scorsese_render::audio::{CHANNELS, mixdown};
use scorsese_render::testing::tone_graph;
use scorsese_render::{FrameRange, Plan, RenderSettings, Resolution, Tools};

use crate::common::audio::RATE;
use crate::common::ffmpeg::generate;

mod line;
mod openings;

/// Where the line on the other track starts and ends, in seconds: two seams,
/// both well inside the bed.
pub(super) const SEAMS: [f64; 2] = [1.0, 2.0];

/// What a near-silent sample is, against a bed peaking at a half.
const NEAR_SILENT: f32 = 0.01;

pub(super) const MP3: &[&str] = &["-c:a", "libmp3lame", "-b:a", "128k"];
pub(super) const M4A: &[&str] = &["-c:a", "aac", "-b:a", "128k"];

/// Renders `project`'s mix and holds it against one decode of the whole bed,
/// read from `offset` seconds in: no more near-silence after each of `seams`
/// (in mix seconds) than that decode has there, and the same samples
/// throughout.
pub(super) fn holds_to_one_decode(
    tools: &Tools,
    project: &Project,
    dir: &Path,
    extension: &str,
    offset: f64,
    seams: &[f64],
) {
    let mixed = mix_of(tools, project, dir);
    let whole = one_decode(tools, &dir.join(format!("assets/bed.{extension}")));
    let whole = &whole[(offset * f64::from(RATE)) as usize * CHANNELS..];
    assert!(
        mixed.len() <= whole.len(),
        "the mix ({} samples) outran the file ({})",
        mixed.len(),
        whole.len()
    );

    for &seam in seams {
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
        .zip(whole)
        .enumerate()
        .map(|(i, (a, b))| (i, (a - b).abs()))
        .fold((0, 0.0_f32), |w, d| if d.1 > w.1 { d } else { w });
    assert!(
        worst.1 < 1e-5,
        "{extension}: the mix departs from one decode by {} at {:.4} s",
        worst.1,
        (worst.0 / CHANNELS) as f64 / f64::from(RATE)
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A five-second sine at a half, at 44.1 kHz as Pixabay's music is, encoded
/// with `codec` into `assets/bed.{extension}`. It outlasts every mix of it: an
/// `.m4a`'s alignment can move a few thousandths at the file's own end, which
/// is not a seam.
pub(super) fn compressed_bed(tools: &Tools, root: &Path, extension: &str, codec: &[&str]) -> Asset {
    let graph = format!("{},aresample=44100", tone_graph(440, "0.5", 5.0));
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
fn mix_of(tools: &Tools, project: &Project, root: &Path) -> Vec<f32> {
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
