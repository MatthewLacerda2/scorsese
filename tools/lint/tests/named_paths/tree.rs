//! The rule over a real checkout: what is on disk is what exists, staged or
//! not, and a path git ignores passes only where its directory exists.

use std::fs;
use std::path::{Path, PathBuf};

use scorsese_lint::rules::ignored::git;
use scorsese_lint::rules::named_paths::check_root;

/// A throwaway git checkout with `files` staged, rebuilt each run.
fn checkout(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("mkdir");
    let run = |args: &[&str]| {
        let status = git()
            .args(args)
            .current_dir(&root)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    };
    run(&["init", "--quiet"]);
    for (path, text) in files {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        fs::write(&full, text).expect("write");
        run(&["add", path]);
    }
    root
}

#[test]
fn the_disk_decides_and_an_ignored_path_needs_its_directory() {
    let doc = "docs/new.md, docs/gone.md, deploy/.env and docs/cache/x.md\n";
    let root = checkout(
        "named_paths_disk",
        &[
            (".gitignore", "cache/\n.env\n"),
            ("docs/a.md", doc),
            ("docs/gone.md", "\n"),
            ("deploy/compose.yaml", "\n"),
        ],
    );
    fs::write(root.join("docs/new.md"), "\n").expect("an unstaged file");
    fs::remove_file(root.join("docs/gone.md")).expect("a file deleted on disk");
    let named: Vec<String> = check_root(&root)
        .into_iter()
        .filter_map(|finding| finding.message.split('`').nth(1).map(str::to_owned))
        .collect();
    assert_eq!(named, ["docs/gone.md", "docs/cache/x.md"]);
}
