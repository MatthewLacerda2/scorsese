//! Hand-written JavaScript is measured like TypeScript (#1205), and what
//! nobody wrote by hand — a vendored minified build, a bundler's or a test
//! runner's output — is not.

use std::path::Path;

use scorsese_lint::classify::{Kind, classify, is_skipped_dir};
use scorsese_lint::scan::code_lines;

fn kind_of(path: &str) -> Option<Kind> {
    classify(Path::new(path))
}

#[test]
fn the_motion_kit_and_the_capture_scripts_are_source() {
    for path in [
        "crates/render/src/page/shipped/kit/01-easing.js",
        "crates/render/src/page/layout.js",
        "crates/render/src/page/clock.js",
        "web/landing/hero.scor/pages/out.js",
    ] {
        assert_eq!(kind_of(path), Some(Kind::Source), "{path}");
    }
}

#[test]
fn a_javascript_test_is_named_as_a_typescript_one_is() {
    assert_eq!(kind_of("web/src/x.test.js"), Some(Kind::Test));
    assert_eq!(kind_of("web/tests/e2e/x.js"), Some(Kind::Test));
    assert_eq!(kind_of("web/src/latest.js"), Some(Kind::Source));
}

#[test]
fn a_vendored_minified_build_is_not_measured_by_its_name() {
    for path in [
        "crates/render/src/page/shipped/anime.min.js",
        "crates/render/src/page/shipped/lottie.min.js",
        "crates/render/src/page/shipped/opentype.min.js",
    ] {
        assert_eq!(kind_of(path), None, "{path}");
    }
    // The name, not a resemblance: `min` elsewhere in it is still ours.
    assert_eq!(kind_of("web/src/min.js"), Some(Kind::Source));
    assert_eq!(kind_of("web/src/admin.js"), Some(Kind::Source));
}

#[test]
fn build_and_test_output_is_skipped_only_where_a_tool_would_write_it() {
    for dir in ["web/dist", "web/test-results", "web/playwright-report"] {
        assert!(is_skipped_dir(Path::new(dir)), "{dir}");
        assert_eq!(kind_of(&format!("{dir}/assets/index-abc.js")), None);
    }
    // A module of that name is code like any other.
    assert!(!is_skipped_dir(Path::new("web/src/dist")));
    assert_eq!(kind_of("web/src/dist/x.ts"), Some(Kind::Source));
}

#[test]
fn javascript_comments_are_free_as_typescript_ones_are() {
    let kit = "// A header.\n(() => {\n  /** A doc. */\n  const a = 1;\n\n})();\n";
    assert_eq!(code_lines(kit.as_bytes()), 3);
}
