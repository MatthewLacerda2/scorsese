//! Over a checkout: only tracked files are read, and a skill folder without
//! a `SKILL.md` is found.

use std::fs;
use std::path::{Path, PathBuf};

use scorsese_lint::check_tree;
use scorsese_lint::rules::agent_docs::{CLAUDE_MD, CLAUDE_MD_LIMIT, skill};
use scorsese_lint::rules::ignored::git;

use super::{lines, skill_md};

/// A throwaway git checkout with `tracked` staged and `loose` only written.
fn checkout(name: &str, tracked: &[(&str, String)], loose: &[(&str, String)]) -> PathBuf {
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
    for (path, text) in tracked.iter().chain(loose) {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        fs::write(&full, text).expect("write");
    }
    for (path, _) in tracked {
        run(&["add", path]);
    }
    root
}

/// The agent-doc findings under `root`, as `(path, rule)`.
fn found(root: &Path) -> Vec<(PathBuf, &'static str)> {
    check_tree(root)
        .expect("walk")
        .into_iter()
        .filter(|finding| [CLAUDE_MD, skill::RULE].contains(&finding.rule))
        .map(|finding| (finding.path, finding.rule))
        .collect()
}

#[test]
fn a_tracked_claude_md_over_its_cap_is_found_and_an_untracked_one_is_not() {
    let over = lines(CLAUDE_MD_LIMIT + 1);
    let root = checkout(
        "agent_docs_tracked",
        &[("crates/core/CLAUDE.md", over.clone())],
        &[("scratch/CLAUDE.md", over)],
    );
    assert_eq!(
        found(&root),
        [(PathBuf::from("crates/core/CLAUDE.md"), CLAUDE_MD)]
    );
}

#[test]
fn a_skill_folder_without_its_skill_md_is_found() {
    let root = checkout(
        "agent_docs_folders",
        &[
            (
                ".claude/skills/merge/SKILL.md",
                skill_md("name: merge\ndescription: Merge."),
            ),
            (".claude/skills/orphan/notes.md", "# Notes\n".into()),
        ],
        &[],
    );
    assert_eq!(
        found(&root),
        [(PathBuf::from(".claude/skills/orphan"), skill::RULE)]
    );
}
