//! Reading a finished file back, and comparing two of them.
//!
//! Real files through real ffmpeg, because that is the whole claim: the same
//! numbers a render works out as it builds a mix can be recovered from
//! something already on disk, whatever container it came in.

use std::path::{Path, PathBuf};

use scorsese_render::audio::{Profile, measure};
use scorsese_render::{Tools, say};
use scorsese_soundgen::level::Difference;

use crate::common::ffmpeg::{fixture_dir, generate, tools};

use super::SLACK;

/// A file of `seconds` of a sine, whose amplitude is whatever `expression`
/// evaluates to — a constant, or something that changes partway through.
fn wav(tools: &Tools, dir: &Path, name: &str, amplitude: &str, seconds: f64) -> PathBuf {
    let file = dir.join(format!("{name}.wav"));
    let graph = format!(
        "aevalsrc=exprs=({amplitude})*sin(2*PI*440*t):duration={seconds}:sample_rate=48000"
    );
    generate(tools, &file, &["-f", "lavfi", "-i", &graph]);
    file
}

fn mean_of(profile: &Profile) -> f64 {
    profile.whole.loudness.mean_dbfs.expect("audible")
}

/// The level of a file on disk is the level of the signal that was written into
/// it: a half-scale sine is 3 dB under its own peak, and nothing about being
/// encoded and decoded moves that.
#[test]
fn a_file_reads_at_the_level_it_was_written_at() {
    let tools = tools();
    let dir = fixture_dir("measure-known");
    let file = wav(&tools, &dir, "half", "0.5", 2.0);

    let profile = measure(&tools, &file).expect("a wav can be measured");
    let expected = 20.0 * (0.5 / std::f64::consts::SQRT_2).log10();
    let mean = mean_of(&profile);
    assert!(
        (mean - expected).abs() < SLACK,
        "a half-scale sine should read near {expected:.1} dBFS, and read {mean:.1}"
    );
    // 440 Hz is midrange, and a report that could not tell where a tone sits
    // could not tell a muddy mix from a thin one either.
    let bands = profile.whole.bands.expect("a tone has a balance");
    assert!(bands.mid > 0.8, "440 Hz is midrange: {bands:?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// The case that has to come out clean, because it is what re-measuring an
/// unchanged file produces.
#[test]
fn a_file_compared_with_itself_differs_by_nothing() {
    let tools = tools();
    let dir = fixture_dir("measure-same");
    let file = wav(&tools, &dir, "same", "0.5", 2.0);

    let profile = measure(&tools, &file).expect("measured");
    let difference = Difference::between(&profile.whole, &profile.whole);
    assert!(difference.is_nothing(), "{difference:?}");
    assert_eq!(
        say::comparison(&profile, &profile),
        vec!["identical — nothing measurably changed".to_owned()]
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The finding the comparison exists for: this one is *this many decibels*
/// under the version it was meant to replace.
#[test]
fn a_quieter_file_reads_as_quieter_than_the_one_it_replaced() {
    let tools = tools();
    let dir = fixture_dir("measure-against");
    let before = measure(&tools, &wav(&tools, &dir, "before", "0.5", 2.0)).expect("measured");
    let after = measure(&tools, &wav(&tools, &dir, "after", "0.25", 2.0)).expect("measured");

    let difference = Difference::between(&after.whole, &before.whole);
    let by = difference.mean_db.expect("audible");
    assert!(
        (by + 6.02).abs() < SLACK,
        "halving the amplitude should read about 6 dB down, and read {by:.1}"
    );
    let said = say::comparison(&after, &before);
    assert!(
        said.iter().any(|line| line.contains("quieter")),
        "the report should say which way it moved: {said:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A file long enough to have parts reports them separately, which is the whole
/// reason for a table: the whole-file mean sits between the two and would have
/// found neither.
#[test]
fn a_file_that_changes_partway_reports_the_change_on_a_row() {
    let tools = tools();
    let dir = fixture_dir("measure-sections");
    // The commas are escaped because a filtergraph separates filters with
    // them; unescaped, ffmpeg reads the expression as three filters.
    let file = wav(&tools, &dir, "halves", r"if(lt(t\,9)\,0.5\,0.125)", 18.0);

    let profile = measure(&tools, &file).expect("measured");
    assert!(
        profile.sections.len() >= 3,
        "eighteen seconds is more than one stretch: {:?}",
        profile.sections.len()
    );
    let mean = |at: usize| profile.sections[at].loudness.mean_dbfs.expect("audible");
    let first = mean(0);
    let last = mean(profile.sections.len() - 1);
    assert!(
        (first - last - 12.04).abs() < SLACK,
        "the quarter-scale tail should read 12 dB under the head: \
         first {first:.1}, last {last:.1}"
    );
    let whole = mean_of(&profile);
    assert!(whole < first && whole > last, "whole {whole:.1}");
    std::fs::remove_dir_all(&dir).ok();
}
