//! A lossy delivery stays under full scale, whatever the codec adds (#503).
//!
//! The fixture is a square wave peaking at −1 dBFS on both channels: every
//! sample of it is inside full scale, and AAC rebuilds its edges with ringing
//! that comes back more than 3 dB over — measured with plain ffmpeg and no
//! scorsese in the loop, the same shape as the dense synth score that found
//! this. Each check reads the delivered file back through the crate's own
//! [`scorsese_render::audio::measure`], never the render's word for it, and
//! with tolerance: the bytes of an encode are not ours to assert on.

use std::path::{Path, PathBuf};

use scorsese_core::{Asset, AssetKind, Fps};
use scorsese_render::audio::{DELIVERY_CEILING_DBTP, measure};
use scorsese_render::{
    Container, FrameRange, OutputFormat, RenderReport, RenderSettings, Renderer, Resolution, Tools,
};

use crate::common::audio::RATE;
use crate::common::ffmpeg::{fixture_dir, generate_asset, tools};
use crate::common::{audio_track, clip, project, video_track};
use crate::picture;

/// −1 dBFS, as a sample value: the ceiling a bake's own limiter holds to.
const LEVEL: f64 = 0.891;

/// Decibels a decoded peak may sit away from where it is expected. The codec is
/// not ours and nor is the resample on the way back out.
const SLACK: f64 = 0.5;

/// A stereo tone peaking at `amplitude`, square when `square` and sine
/// otherwise.
fn tone(tools: &Tools, root: &Path, square: bool, amplitude: f64) -> Asset {
    let wave = if square {
        format!("{amplitude}*sgn(sin(2*PI*220*t))")
    } else {
        format!("{amplitude}*sin(2*PI*440*t)")
    };
    let graph = format!("aevalsrc=exprs={wave}|{wave}:duration=2:sample_rate={RATE}");
    generate_asset(
        tools,
        root,
        "tone",
        AssetKind::Audio,
        &["-f", "lavfi", "-i", &graph],
    )
}

/// Renders the tone over two seconds of picture into `container`.
fn deliver(label: &str, square: bool, amplitude: f64, container: Container) -> Delivered {
    let tools = tools();
    let dir = fixture_dir(label);
    let project = project(
        vec![
            picture(&tools, &dir, 2),
            tone(&tools, &dir, square, amplitude),
        ],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![clip("m1", "tone", 0, 60)]),
        ],
    );
    let out = dir.join(format!("out.{}", container.name()));
    let resolution = Resolution::new(32, 32).expect("a legal raster");
    let settings = RenderSettings::new(resolution, Fps::THIRTY)
        .with_format(OutputFormat::defaults_for(container));
    let report = Renderer::new(&tools, settings)
        .render(&project, &dir, FrameRange::ALL, &out)
        .expect("the render succeeds");
    let loudness = measure(&tools, &out).expect("measured").whole.loudness;
    Delivered {
        dir,
        report,
        peak: loudness.peak_dbfs.expect("audible"),
        true_peak: loudness.true_peak_dbfs.expect("audible"),
    }
}

struct Delivered {
    dir: PathBuf,
    report: RenderReport,
    peak: f64,
    true_peak: f64,
}

impl Drop for Delivered {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

#[test]
fn material_that_overshoots_the_codec_is_delivered_under_full_scale() {
    let it = deliver("headroom-square", true, LEVEL, Container::Mp4);

    let trim = it.report.trim.expect("the square wave needed room");
    assert!(
        trim.encoded_dbtp > 0.0,
        "the fixture has to overshoot to prove anything, and came back at {:+.1} dBTP",
        trim.encoded_dbtp
    );
    assert!(
        it.peak < 0.0,
        "the delivered file peaks at {:+.2} dBFS — over full scale",
        it.peak
    );
    assert!(
        it.true_peak < DELIVERY_CEILING_DBTP + SLACK,
        "the delivered file's true peak is {:+.2} dBTP, over the ceiling",
        it.true_peak
    );
    let reported = it.report.delivered.expect("the report reads the file back");
    assert!(!reported.is_clipping(), "and says it does not clip");
}

#[test]
fn material_that_already_fits_is_delivered_exactly_as_mixed() {
    // A sine at half scale: nowhere near the ceiling after any codec.
    let it = deliver("headroom-sine", false, 0.5, Container::Mp4);

    assert_eq!(it.report.trim, None, "nothing to make room for");
    let expected = 20.0 * 0.5_f64.log10();
    assert!(
        (it.peak - expected).abs() < SLACK,
        "a half-scale sine should come back near {expected:.1} dBFS, and read {:.1}",
        it.peak
    );
}

#[test]
fn a_lossless_delivery_is_never_turned_down() {
    // The same square wave, into PCM: what goes in is what comes out, so it
    // arrives at −1 dBFS and the codec is owed no room at all.
    let it = deliver("headroom-pcm", true, LEVEL, Container::Avi);

    assert_eq!(it.report.trim, None, "a lossless codec overshoots nothing");
    let expected = 20.0 * LEVEL.log10();
    assert!(
        (it.peak - expected).abs() < SLACK,
        "PCM should hand back {expected:.1} dBFS, and read {:.1}",
        it.peak
    );
}
