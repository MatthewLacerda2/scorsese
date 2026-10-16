//! The manifests and the lockfile are found under a root, wherever they sit.

use std::fs;
use std::path::{Path, PathBuf};

use scorsese_lint::rules::boundary::{check_root, manifests};

/// A throwaway tree holding `files`, rebuilt each run.
fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    // The system temp directory: the lint must not walk into its own tree.
    let root = std::env::temp_dir().join(format!("scorsese-lint-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a file has a folder")).expect("creating a folder");
        fs::write(path, text).expect("writing a file");
    }
    root
}

#[test]
fn every_crate_manifest_and_the_lockfile_are_read() {
    let root = tree(
        "boundary-root",
        &[
            (
                "crates/zimmer/Cargo.toml",
                "[package]\nname = \"scorsese-zimmer\"\n[dependencies]\nscorsese-core = \"1\"\n",
            ),
            (
                "tools/icons/Cargo.toml",
                "[package]\nname = \"scorsese-icon-vendor\"\n[dependencies]\nsqlx = \"1\"\n",
            ),
            (
                "app/Cargo.toml",
                "[package]\nname = \"scorsese-app\"\n[dependencies]\nscorsese-cli = \"1\"\n",
            ),
            ("crates/notes/README.md", "no manifest here"),
            ("Cargo.lock", "[[package]]\nname = \"eframe\"\n"),
        ],
    );
    let found: Vec<PathBuf> = manifests(&root).expect("listing");
    assert_eq!(
        found,
        [
            "app/Cargo.toml",
            "crates/zimmer/Cargo.toml",
            "tools/icons/Cargo.toml"
        ]
        .map(PathBuf::from)
    );
    let findings = check_root(&root).expect("checking");
    let paths: Vec<&Path> = findings.iter().map(|f| f.path.as_path()).collect();
    assert_eq!(
        paths,
        [
            "app/Cargo.toml",
            "crates/zimmer/Cargo.toml",
            "tools/icons/Cargo.toml",
            "Cargo.lock"
        ]
        .map(Path::new)
    );
    fs::remove_dir_all(&root).expect("cleaning up");
}

#[test]
fn a_tree_with_no_crates_has_nothing_to_say() {
    let root = tree("boundary-empty", &[("README.md", "nothing")]);
    assert!(check_root(&root).expect("checking").is_empty());
    fs::remove_dir_all(&root).expect("cleaning up");
}
