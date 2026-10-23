//! What an agent loads into its context: every `CLAUDE.md` and every skill.
//!
//! These files are read at the start of every session, so what they cost is
//! paid every time. Three rules hold them:
//!
//! - **`claude-md`** — a file named `CLAUDE.md`, anywhere, is at most
//!   [`CLAUDE_MD_LIMIT`] lines.
//! - **`skill`** — every file in a skill folder (`.claude/skills/<name>/`) is at
//!   most [`SKILL_LIMIT`] lines, and its `SKILL.md` carries the frontmatter
//!   Claude requires to load it ([`skill`]).
//! - **`history`** — neither kind cites an issue, a pull request or a date
//!   ([`history`]): they state the rule, and the issue that made it keeps why.
//!
//! All lines count, blank ones included. Only tracked files are read: what git
//! does not commit is not loaded by anyone else. An exception is an HTML
//! comment, `<!-- lint: allow(<rule>) <why> -->`, on the line or just above.

pub mod history;
pub mod skill;

use std::fs;
use std::path::{Component, Path};

use super::Finding;
use super::shebang::tracked;

/// The `claude-md` rule's name, as an `allow` names it.
pub const CLAUDE_MD: &str = "claude-md";

/// The most lines a `CLAUDE.md` may have.
pub const CLAUDE_MD_LIMIT: usize = 300;

/// The most lines a file in a skill folder may have.
pub const SKILL_LIMIT: usize = 500;

/// Where skills live, relative to the root.
pub const SKILLS: &str = ".claude/skills";

/// What an agent loads, for the rules here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doc<'a> {
    /// A `CLAUDE.md`.
    ClaudeMd,
    /// A file in the skill folder of this name.
    Skill(&'a str),
}

/// Which kind of agent doc `path` is, if any.
pub fn doc(path: &Path) -> Option<Doc<'_>> {
    if path.file_name().is_some_and(|name| name == "CLAUDE.md") {
        return Some(Doc::ClaudeMd);
    }
    let rest = path.strip_prefix(SKILLS).ok()?;
    let mut parts = rest.components();
    let (Some(Component::Normal(folder)), Some(_)) = (parts.next(), parts.next()) else {
        return None;
    };
    folder.to_str().map(Doc::Skill)
}

/// Every finding for one agent doc at `path` with `text`.
pub fn check_doc(path: &Path, text: &str) -> Vec<Finding> {
    let Some(kind) = doc(path) else {
        return Vec::new();
    };
    let lines = text.lines().count();
    let mut findings = Vec::new();
    let finding = |rule, message: String| Finding {
        path: path.to_path_buf(),
        line: None,
        rule,
        message,
    };
    match kind {
        Doc::ClaudeMd if lines > CLAUDE_MD_LIMIT && !excused(text, CLAUDE_MD) => {
            findings.push(finding(
                CLAUDE_MD,
                format!(
                    "{lines} lines, over the {CLAUDE_MD_LIMIT}-line cap: keep what every session \
                     needs; move what one folder needs to that folder's CLAUDE.md and a procedure \
                     to a skill; delete what a check already enforces"
                ),
            ));
        }
        Doc::Skill(_) if lines > SKILL_LIMIT && !excused(text, skill::RULE) => {
            findings.push(finding(
                skill::RULE,
                format!(
                    "{lines} lines, over the {SKILL_LIMIT}-line cap for a skill's file: move a \
                     section into a supporting file beside it, which SKILL.md names"
                ),
            ));
        }
        _ => {}
    }
    if let Doc::Skill(folder) = kind
        && path.file_name().is_some_and(|name| name == "SKILL.md")
    {
        findings.extend(
            skill::frontmatter_findings(text, folder)
                .into_iter()
                .map(|message| Finding {
                    path: path.to_path_buf(),
                    line: Some(1),
                    rule: skill::RULE,
                    message,
                }),
        );
    }
    findings.extend(history::check(path, text));
    findings
}

/// Whether a whole-file rule is excused: an allow for it anywhere in the file.
fn excused(text: &str, rule: &str) -> bool {
    text.lines().any(|line| super::allow::grants(line, rule))
}

/// Every finding for every tracked agent doc under `root`, and a skill folder
/// that has no `SKILL.md`.
pub fn check(root: &Path) -> Vec<Finding> {
    let files: Vec<_> = tracked(root).into_iter().map(|(path, _)| path).collect();
    let mut findings = Vec::new();
    let mut folders: Vec<&str> = Vec::new();
    for path in &files {
        if let Some(Doc::Skill(folder)) = doc(path)
            && !folders.contains(&folder)
        {
            folders.push(folder);
        }
        if doc(path).is_none() {
            continue;
        }
        let Ok(bytes) = fs::read(root.join(path)) else {
            continue;
        };
        findings.extend(check_doc(path, &String::from_utf8_lossy(&bytes)));
    }
    for folder in folders {
        let entry = Path::new(SKILLS).join(folder).join("SKILL.md");
        if !files.contains(&entry) {
            findings.push(Finding {
                path: Path::new(SKILLS).join(folder),
                line: None,
                rule: skill::RULE,
                message: "a skill folder with no SKILL.md, which is what Claude loads".into(),
            });
        }
    }
    findings
}
