//! The golden renders themselves.
//!
//! One test per fixture, so a failure names the fixture that broke and the
//! fixtures run in parallel. Adding a fixture means adding its directory and
//! its name to the list below — and if you forget the second half,
//! `every_fixture_is_covered` fails rather than letting an untested fixture sit
//! in the repository looking like coverage.
//!
//! These need ffmpeg on PATH, which CI installs. They do not skip themselves
//! when it is absent: a correctness gate that passes by doing nothing is worse
//! than no gate.
//!
//! They do skip themselves off Linux, which is a different claim and the only
//! one of its kind here. References are blessed on Linux and CI compares them
//! on Linux; everywhere else the comparison measures the local ffmpeg's decode
//! path as much as it measures the compositor, and the tolerances were never
//! sized for that. So the fixtures are `#[ignore]`d on any other target — not
//! compiled out, so they still have to build and can still be run on purpose
//! with `--run-ignored all`, and reported as *skipped* by the runner rather
//! than passing silently. `docs/golden-renders.md` holds the whole reasoning,
//! including why this is keyed on the platform and not on a decoder mismatch.

use std::path::{Path, PathBuf};

use scorsese_golden::{Mode, assert_matches};

/// Where the fixtures live, and where the harness may make a mess.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
const WORKSPACE: &str = env!("CARGO_TARGET_TMPDIR");

fn check(name: &str) {
    let outcome = assert_matches(
        &Path::new(FIXTURES).join(name),
        Mode::from_env(),
        Path::new(WORKSPACE),
    );
    if !outcome.blessed.is_empty() {
        println!(
            "blessed {} reference frame(s) of `{}`: {:?}",
            outcome.blessed.len(),
            outcome.name,
            outcome.blessed
        );
        return;
    }
    // Printed on success too, so a fixture creeping towards its tolerance is
    // visible in the log before it starts failing.
    for (frame, difference) in &outcome.compared {
        println!(
            "{name} frame {frame}: ssim {:.4}, mean error {:.3}",
            difference.ssim, difference.mean_error
        );
    }
}

macro_rules! goldens {
    ($($name:ident),* $(,)?) => {
        /// Every fixture directory named here, for the coverage check.
        const NAMED: &[&str] = &[$(stringify!($name)),*];

        $(
            #[test]
            // Off Linux the frames are not ours to judge — see the module doc.
            // `ignore` rather than `cfg` so the runner counts them and says so.
            #[cfg_attr(
                not(target_os = "linux"),
                ignore = "the pixel gate is authoritative only on Linux, where the references were blessed"
            )]
            fn $name() {
                check(stringify!($name));
            }
        )*
    };
}

goldens!(
    alpha,
    alpha_scaled,
    anchored,
    arrows,
    attached,
    blend,
    blur_alpha,
    blur_heavy,
    blur_soft,
    captioned,
    crop,
    cuts,
    drawn_weight,
    fade,
    fill,
    flip,
    gap_above,
    grade_brightness,
    grade_contrast,
    grade_ramp,
    grade_saturation,
    grade_temperature,
    grade_vignette,
    icons,
    italic,
    letterbox,
    letterbox_anchored,
    native,
    overlay,
    paragraph,
    resume,
    serif,
    slice,
    slide,
    shapes,
    slugs,
    speed,
    spin,
    title,
    title_moved,
    wash,
    weight,
    zoom
);

#[test]
fn every_fixture_is_covered() {
    let mut found: Vec<String> = std::fs::read_dir(FIXTURES)
        .expect("the fixtures directory exists")
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();

    let mut named: Vec<String> = NAMED.iter().map(|name| (*name).to_owned()).collect();
    named.sort();

    assert_eq!(
        found, named,
        "every fixture directory needs a test in the goldens! list"
    );
}

#[test]
fn a_fixture_that_disagrees_with_itself_is_a_broken_fixture() {
    // The harness's own errors have to distinguish "the render is wrong" from
    // "the fixture is wrong", because only one of those is a real regression.
    let nowhere = PathBuf::from(FIXTURES).join("does-not-exist");
    let error = scorsese_golden::run(&nowhere, Mode::Check, Path::new(WORKSPACE))
        .expect_err("a missing fixture cannot pass");
    assert!(
        matches!(error, scorsese_golden::GoldenError::Fixture(_)),
        "got {error}"
    );
}
