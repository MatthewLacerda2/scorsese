//! A render asked for a loudness target delivers at it (#968).
//!
//! Each check reads the delivered file back through ffmpeg's own `ebur128`
//! meter, never the render's word for it: the report's numbers come from the
//! crate's meter, and the claim worth testing is that they agree with an
//! implementation that is not ours.

use std::path::Path;

use scorsese_core::{AssetKind, Fps};
use scorsese_render::audio::{DELIVERY_CEILING_DBTP, measure};
use scorsese_render::{
    Container, FrameRange, LoudnessTarget, OutputFormat, RenderReport, RenderSettings, Renderer,
    Resolution, Tools,
};

use crate::common::audio::RATE;
use crate::common::ffmpeg::{fixture_dir, generate_asset, tools};
use crate::common::{audio_track, clip, project};

/// LU a delivered file may sit from its target, measured by somebody else's
/// meter through a codec and a resample.
const SLACK: f64 = 0.5;

/// Renders three seconds of the sound `expr` makes, sound only, into
/// `container`, at `target` LUFS. Hands back the report, ffmpeg's reading of
/// the file's integrated loudness, and its true peak.
fn deliver(label: &str, expr: &str, container: Container, target: f64) -> (RenderReport, f64, f64) {
    let tools = tools();
    let dir = fixture_dir(label);
    let graph = format!("aevalsrc=exprs={expr}|{expr}:duration=3:sample_rate={RATE}");
    let sound = generate_asset(
        &tools,
        &dir,
        "sound",
        AssetKind::Audio,
        &["-f", "lavfi", "-i", &graph],
    );
    let project = project(
        vec![sound],
        vec![audio_track("a1", vec![clip("m1", "sound", 0, 90)])],
    );
    let out = dir.join(format!("out.{}", container.name()));
    let target = LoudnessTarget::new(target).expect("a target in range");
    let settings = RenderSettings::new(Resolution::HD, Fps::THIRTY)
        .with_format(OutputFormat::defaults_for(container))
        .with_loudness(Some(target));
    let report = Renderer::new(&tools, settings)
        .render(&project, &dir, FrameRange::ALL, &out)
        .expect("the render succeeds");
    let integrated = ebur128(&tools, &out);
    let true_peak = measure(&tools, &out)
        .expect("measured")
        .whole
        .loudness
        .true_peak_dbfs;
    std::fs::remove_dir_all(&dir).ok();
    (report, integrated, true_peak.unwrap_or(f64::NEG_INFINITY))
}

/// The integrated loudness ffmpeg's `ebur128` filter reads from `file`.
fn ebur128(tools: &Tools, file: &Path) -> f64 {
    let output = tools
        .ffmpeg()
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(file)
        .args(["-af", "ebur128", "-f", "null", "-"])
        .output()
        .expect("run ffmpeg");
    let said = String::from_utf8_lossy(&output.stderr);
    let summary = said.rsplit("Summary:").next().expect("a summary");
    summary
        .lines()
        .find_map(|line| line.trim().strip_prefix("I:"))
        .and_then(|value| value.trim().trim_end_matches("LUFS").trim().parse().ok())
        .unwrap_or_else(|| panic!("no integrated loudness in: {summary}"))
}

#[test]
fn a_quiet_mix_is_brought_up_to_the_target() {
    let (report, read, _) = deliver("lift-quiet", "0.05*sin(2*PI*440*t)", Container::M4a, -16.0);
    let lift = report.lift.expect("a target was asked for");
    assert!(lift.gain_db > 5.0, "raised: {lift}");
    assert!((read + 16.0).abs() < SLACK, "ffmpeg reads {read} LUFS");
    let said = lift.delivered_lufs.expect("measured");
    assert!(
        (said - read).abs() < 0.2,
        "the report says {said}, ffmpeg {read}"
    );
}

/// A quiet bed with loud clicks on it: raising the bed to the target pushes
/// the clicks far past the ceiling, and the limiter holds them there.
#[test]
fn peaks_the_gain_pushes_over_are_held_under_the_ceiling() {
    let expr = "0.05*sin(2*PI*440*t)+0.8*lt(mod(t\\,0.25)\\,0.0001)";
    let (report, read, true_peak) = deliver("lift-clicks", expr, Container::Wav, -14.0);
    let lift = report.lift.expect("a target was asked for");
    assert!(lift.limited_db > 3.0, "the clicks were limited: {lift}");
    assert!((read + 14.0).abs() < SLACK, "ffmpeg reads {read} LUFS");
    assert!(
        true_peak <= DELIVERY_CEILING_DBTP + 0.05,
        "true peak {true_peak} dBTP"
    );
    assert_eq!(report.trim, None, "a lossless delivery needs no room");
}

#[test]
fn a_loud_mix_is_brought_down_without_limiting() {
    let (report, read, _) = deliver("lift-down", "0.9*sin(2*PI*440*t)", Container::Mp3, -30.0);
    let lift = report.lift.expect("a target was asked for");
    assert!(lift.gain_db < -20.0, "lowered: {lift}");
    assert!(lift.limited_db < 0.05, "nothing to hold: {lift}");
    assert!((read + 30.0).abs() < SLACK, "ffmpeg reads {read} LUFS");
}

#[test]
fn silence_has_no_loudness_to_bring_anywhere() {
    let (report, _, _) = deliver("lift-silent", "0", Container::Wav, -14.0);
    let lift = report.lift.expect("a target was asked for");
    assert_eq!(lift.mixed_lufs, None);
    assert_eq!(lift.gain_db, 0.0);
    assert!(lift.to_string().contains("silent"), "{lift}");
}

/// A tone with bursts of square wave on it, raised into AAC: the bursts are
/// limited, the codec overshoots their edges, and the limiter's ceiling comes
/// down by that room instead of the whole mix being trimmed under its target.
#[test]
fn a_lossy_codecs_room_comes_out_of_the_peaks_not_the_target() {
    let expr = "0.2*sin(2*PI*440*t)+0.5*sgn(sin(2*PI*220*t))*lt(mod(t\\,0.5)\\,0.05)";
    let (report, read, true_peak) = deliver("lift-aac", expr, Container::M4a, -10.0);
    let lift = report.lift.expect("a target was asked for");
    assert!(lift.ceiling_dbtp < DELIVERY_CEILING_DBTP, "lowered: {lift}");
    assert!(
        (read + 10.0).abs() < SLACK,
        "ffmpeg reads {read} LUFS: {lift:?}"
    );
    assert!(
        true_peak < DELIVERY_CEILING_DBTP + SLACK,
        "true peak {true_peak}"
    );
}

/// A mix whose loudness is all in its peaks cannot be made much louder
/// without crushing them, so the render stops at six decibels of make-up and
/// says how far short it fell.
#[test]
fn a_target_out_of_the_mixs_reach_is_said_to_be() {
    let expr = "0.02*sin(2*PI*440*t)+0.6*sgn(sin(2*PI*220*t))*lt(mod(t\\,0.5)\\,0.02)";
    let (report, read, _) = deliver("lift-short", expr, Container::Wav, -10.0);
    let lift = report.lift.expect("a target was asked for");
    let planned = -10.0 - lift.mixed_lufs.expect("measured");
    assert!(lift.gain_db <= planned + 6.0 + 1e-9, "{lift:?}");
    assert!(read < -11.0, "ffmpeg reads {read} LUFS");
    assert!(lift.to_string().contains("short"), "{lift}");
}
