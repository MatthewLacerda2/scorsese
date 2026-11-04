//! A repository path a comment or a doc names must exist (#1617).
//!
//! A path left naming a file that moved or never landed reads as true and
//! sends the next agent to nothing. So what an agent reads as description is
//! scanned for tokens shaped like repository paths, and each is looked up
//! among the tracked files and their directories:
//!
//! - **What is read** ([`syntax`]): comment lines in every `.rs`; `#`
//!   comments in the `.py`, `.sh` and `.yml` under `.github/`; the root
//!   `Makefile`'s comments and its `## ` help text; and every line of the
//!   Markdown under `docs/` and `.claude/`, and of every `CLAUDE.md` and
//!   `README.md`. String literals in code are never read, so a test's
//!   invented fixture path is not a hit.
//! - **What is a path** ([`named_in`]): a token that starts with a known root
//!   ([`ROOTS`]), or with a crate's name followed by `src/` or `tests/`
//!   (read under `crates/`), and ends in a file extension or `/`. Markdown's
//!   emphasis and brackets around it (`**docs/mcp.md**`, `<docs/mcp.md>`) are
//!   trimmed; a token still holding a template or glob character
//!   (`<crate>`, `*`, `{a,b}`, `…`, `$`), a `..`, or a leading `/` names no
//!   one file, and is skipped.
//! - **What exists** ([`Known`]): a file on disk that git tracks or would
//!   track (so an unstaged new file counts, and a deleted one does not), a
//!   directory holding one, or a path git ignores whose directory exists
//!   (`deploy/.env`, `app/target/`): made on the machine, never committed, so
//!   named on purpose.
//!
//! Symbol names (`audio::speech`) and prose claims are out of reach; a reader
//! catches those. A path named on purpose though absent from the tree (a file
//! a command creates, an example) is excused inline: `lint: allow(named-path)
//! <why>` in a comment on the line or the one just above it, or in Markdown
//! an HTML comment on the line or in the run of them just above it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::Finding;
use super::ignored::{git, ignored};
use super::shebang::tracked;
use super::{agent_docs, allow};

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "named-path";

/// The top-level directories a named path may start with.
pub const ROOTS: [&str; 8] = [
    "crates", "app", "web", "deploy", "tools", "docs", ".github", ".claude",
];

/// Characters that make a token a template or a glob rather than a path.
const TEMPLATE: &[char] = &['*', '?', '{', '}', '<', '>', '$', '…', '~', '\\', '%'];

/// What Markdown wraps a path in, trimmed from a token's ends.
const WRAPPING: &[char] = &['*', '_', '?', '<', '>'];

/// What exists: the tracked files, their directories, and the crates.
#[derive(Debug, Clone, Default)]
pub struct Known {
    files: BTreeSet<PathBuf>,
    dirs: BTreeSet<PathBuf>,
    crates: BTreeSet<String>,
    ignored: BTreeSet<String>,
}

impl Known {
    /// The tree made of these files, relative to the root.
    pub fn new<P: AsRef<Path>>(files: impl IntoIterator<Item = P>) -> Self {
        let mut known = Self::default();
        for file in files {
            let file = file.as_ref();
            for dir in file.ancestors().skip(1) {
                if dir.as_os_str().is_empty() {
                    break;
                }
                if let Ok(rest) = dir.strip_prefix("crates")
                    && let Some(name) = rest.to_str().filter(|name| !name.contains('/'))
                    && !name.is_empty()
                {
                    known.crates.insert(name.to_owned());
                }
                known.dirs.insert(dir.to_path_buf());
            }
            known.files.insert(file.to_path_buf());
        }
        known
    }

    /// Count these paths, which git ignores, as existing where their
    /// directory does.
    pub fn ignore<S: Into<String>>(&mut self, paths: impl IntoIterator<Item = S>) {
        self.ignored.extend(paths.into_iter().map(Into::into));
    }

    /// Whether `path` (a file, or a directory with a trailing `/`) exists.
    pub fn contains(&self, path: &str) -> bool {
        let parent = Path::new(path.trim_end_matches('/')).parent();
        let placed = parent.is_none_or(|dir| dir.as_os_str().is_empty() || self.dirs.contains(dir));
        if self.ignored.contains(path) && placed {
            return true;
        }
        match path.strip_suffix('/') {
            Some(dir) => self.dirs.contains(Path::new(dir)),
            None => self.files.contains(Path::new(path)) || self.dirs.contains(Path::new(path)),
        }
    }
}

/// How a file's description is found in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// `//` comment lines.
    Rust,
    /// `#` comment lines.
    Hash,
    /// `#` comment lines, and the help text after a target's `## `.
    Makefile,
    /// Every line.
    Markdown,
}

/// How `path` is read, or `None` when the rule does not read it.
pub fn syntax(path: &Path) -> Option<Syntax> {
    let name = path.file_name()?.to_str()?;
    let extension = path.extension().and_then(|ext| ext.to_str());
    let under = |root: &str| path.starts_with(root);
    match extension {
        Some("rs") => Some(Syntax::Rust),
        Some("py" | "sh" | "yml" | "yaml") if under(".github") => Some(Syntax::Hash),
        Some("md") if matches!(name, "CLAUDE.md" | "README.md") => Some(Syntax::Markdown),
        Some("md") if under("docs") || under(".claude") => Some(Syntax::Markdown),
        None if path == Path::new("Makefile") => Some(Syntax::Makefile),
        _ => None,
    }
}

/// The part of `line` that is description, in a file of `syntax`.
fn described(line: &str, syntax: Syntax) -> Option<&str> {
    let trimmed = line.trim_start();
    let hash = || (trimmed.starts_with('#') && !trimmed.starts_with("#!")).then_some(trimmed);
    match syntax {
        Syntax::Rust => trimmed.starts_with("//").then_some(trimmed),
        Syntax::Hash => hash(),
        Syntax::Makefile => hash().or_else(|| line.split_once(" ## ").map(|(_, help)| help)),
        Syntax::Markdown => Some(line),
    }
}

/// The repository paths `text` names, each as it is read from the root.
pub fn named_in(text: &str, known: &Known) -> Vec<String> {
    let is_part = |c: char| c.is_alphanumeric() || "_-./".contains(c) || TEMPLATE.contains(&c);
    text.split(|c: char| !is_part(c))
        .map(|token| token.trim_matches(WRAPPING))
        .filter(|token| {
            !token.contains(TEMPLATE) && !token.contains("..") && !token.starts_with('/')
        })
        .map(|token| token.trim_end_matches('.'))
        .filter_map(|token| resolve(token, known))
        .collect()
}

/// The path `token` names from the root, if it is shaped like one.
fn resolve(token: &str, known: &Known) -> Option<String> {
    let (first, rest) = token.split_once('/')?;
    let path = if ROOTS.contains(&first) {
        token.to_owned()
    } else if known.crates.contains(first)
        && (rest.starts_with("src/") || rest.starts_with("tests/"))
    {
        format!("crates/{token}")
    } else {
        return None;
    };
    let last = path.rsplit('/').next().unwrap_or_default();
    let extension = last.rsplit_once('.').map(|(_, ext)| ext);
    // Never empty: a token's closing `.` is trimmed before it gets here.
    let file_like = extension.is_some_and(|ext| ext.chars().all(char::is_alphanumeric));
    (path.ends_with('/') || file_like).then_some(path)
}

/// Each missing path `text`, the file at `path`, names, with its 0-based
/// line, unless an allow excuses it.
fn missing(path: &Path, text: &str, known: &Known) -> Vec<(usize, String)> {
    let Some(syntax) = syntax(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    let excused = |index: usize| match syntax {
        Syntax::Markdown => agent_docs::allowed(&lines, index, RULE),
        _ => (index.saturating_sub(1)..=index).any(|at| allow::grants(lines[at], RULE)),
    };
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(description) = described(line, syntax) else {
            continue;
        };
        for named in named_in(description, known) {
            if !known.contains(&named) && !excused(index) {
                found.push((index, named));
            }
        }
    }
    found
}

/// A finding for every missing path `text`, the file at `path`, names.
pub fn check(path: &Path, text: &str, known: &Known) -> Vec<Finding> {
    missing(path, text, known)
        .into_iter()
        .map(|(index, named)| Finding {
            path: path.to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: format!(
                "names `{named}`, which does not exist: repoint it to where that file is \
                 now, or remove it (or excuse it inline if it is named on purpose, such \
                 as a file a command creates)"
            ),
        })
        .collect()
}

/// The files git tracks or would track under `root` that are on disk.
fn present(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = tracked(root).into_iter().map(|(path, _)| path).collect();
    let untracked = git()
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .current_dir(root)
        .output();
    if let Some(output) = untracked.ok().filter(|output| output.status.success()) {
        let listed = String::from_utf8_lossy(&output.stdout).into_owned();
        files.extend(
            listed
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from),
        );
    }
    files.retain(|path| root.join(path).is_file());
    files
}

/// Every finding for every file under `root` the rule reads.
pub fn check_root(root: &Path) -> Vec<Finding> {
    let files = present(root);
    let mut known = Known::new(&files);
    let texts: Vec<(&PathBuf, String)> = files
        .iter()
        .filter(|path| syntax(path).is_some())
        .filter_map(|path| Some((path, fs::read(root.join(path)).ok()?)))
        .map(|(path, bytes)| (path, String::from_utf8_lossy(&bytes).into_owned()))
        .collect();
    let absent: BTreeSet<PathBuf> = texts
        .iter()
        .flat_map(|(path, text)| missing(path, text, &known))
        .map(|(_, named)| PathBuf::from(named))
        .collect();
    let absent: Vec<&Path> = absent.iter().map(PathBuf::as_path).collect();
    known.ignore(
        ignored(root, &absent)
            .iter()
            .map(|path| path.to_string_lossy().into_owned()),
    );
    texts
        .iter()
        .flat_map(|(path, text)| check(path, text, &known))
        .collect()
}
