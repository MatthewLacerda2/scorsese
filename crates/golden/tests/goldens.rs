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
            fn $name() {
                check(stringify!($name));
            }
        )*
    };
}

goldens!(
    blend,
    cuts,
    fade,
    fill,
    gap_above,
    letterbox,
    native,
    overlay,
    paragraph,
    resume,
    serif,
    slice,
    slide,
    slugs,
    spin,
    title,
    title_moved,
    wash,
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
