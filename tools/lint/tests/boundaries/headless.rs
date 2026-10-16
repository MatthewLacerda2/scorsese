//! No display crate in the root workspace's lockfile.

use scorsese_lint::rules::boundary::headless::{DISPLAY, RULE, check, display_crate};

#[test]
fn every_display_crate_and_its_family_is_one() {
    for crate_ in DISPLAY {
        assert_eq!(display_crate(crate_), Some(crate_));
    }
    assert_eq!(display_crate("egui_extras"), Some("egui"));
    assert_eq!(display_crate("wgpu-core"), Some("wgpu"));
}

#[test]
fn a_name_that_only_starts_the_same_is_not_one() {
    for package in ["glowworm", "rfdx", "cpalette", "serde", "eguide"] {
        assert_eq!(display_crate(package), None, "{package}");
    }
}

#[test]
fn a_lockfile_with_a_window_in_it_is_flagged_at_the_package() {
    let lock = "[[package]]\nname = \"serde\"\nversion = \"1\"\n\n[[package]]\nname = \"winit\"\nversion = \"0.30\"\n";
    let [finding] = &check(lock)[..] else {
        panic!("expected one finding");
    };
    assert_eq!((finding.rule, finding.line), (RULE, Some(6)));
    assert!(check("[[package]]\nname = \"serde\"\n").is_empty());
}
