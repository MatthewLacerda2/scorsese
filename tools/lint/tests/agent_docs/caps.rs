//! The line caps: every `CLAUDE.md` at 300, every skill file at 500.

use std::path::Path;

use scorsese_lint::rules::agent_docs::{
    CLAUDE_MD, CLAUDE_MD_LIMIT, Doc, SKILL_LIMIT, check_doc, doc, skill,
};

use super::{lines, rules};

#[test]
fn a_claude_md_anywhere_is_an_agent_doc() {
    assert_eq!(doc(Path::new("CLAUDE.md")), Some(Doc::ClaudeMd));
    assert_eq!(doc(Path::new("crates/core/CLAUDE.md")), Some(Doc::ClaudeMd));
    assert_eq!(
        doc(Path::new(".claude/skills/ci-merge/SKILL.md")),
        Some(Doc::Skill("ci-merge"))
    );
    assert_eq!(
        doc(Path::new(".claude/skills/issue-batch/cloud-brief.md")),
        Some(Doc::Skill("issue-batch"))
    );
    assert_eq!(doc(Path::new("docs/web.md")), None);
    assert_eq!(doc(Path::new(".claude/skills/README.md")), None);
}

#[test]
fn a_claude_md_over_its_cap_is_flagged_and_one_at_it_passes() {
    for path in ["CLAUDE.md", "crates/server/CLAUDE.md"] {
        let over = check_doc(Path::new(path), &lines(CLAUDE_MD_LIMIT + 1));
        assert_eq!(rules(&over), [CLAUDE_MD], "{path}");
        assert!(over[0].message.contains("301 lines"));
        assert!(check_doc(Path::new(path), &lines(CLAUDE_MD_LIMIT)).is_empty());
    }
}

#[test]
fn blank_lines_count_toward_the_cap() {
    let text = format!("{}{}", lines(150), "\n".repeat(151));
    assert_eq!(
        rules(&check_doc(Path::new("CLAUDE.md"), &text)),
        [CLAUDE_MD]
    );
}

#[test]
fn a_skill_file_over_its_cap_is_flagged_and_one_at_it_passes() {
    let path = Path::new(".claude/skills/issue-batch/cloud-brief.md");
    assert_eq!(
        rules(&check_doc(path, &lines(SKILL_LIMIT + 1))),
        [skill::RULE]
    );
    assert!(check_doc(path, &lines(SKILL_LIMIT)).is_empty());
}

#[test]
fn a_whole_file_allow_with_a_reason_excuses_the_cap() {
    let excused = format!(
        "<!-- lint: allow({CLAUDE_MD}) a vendored file read as it is -->\n{}",
        lines(CLAUDE_MD_LIMIT)
    );
    assert!(check_doc(Path::new("CLAUDE.md"), &excused).is_empty());
    let bare = format!(
        "<!-- lint: allow({CLAUDE_MD}) -->\n{}",
        lines(CLAUDE_MD_LIMIT)
    );
    assert_eq!(
        rules(&check_doc(Path::new("CLAUDE.md"), &bare)),
        [CLAUDE_MD]
    );
}

#[test]
fn other_markdown_is_not_read() {
    assert!(check_doc(Path::new("docs/web.md"), &lines(5000)).is_empty());
}
