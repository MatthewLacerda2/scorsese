//! Words a part of the tree may not write, outside the one place that may.
//!
//! Each [`Fence`] is a boundary a crate doc states about its source: zimmer
//! and guide do no I/O, the compositor and core none they disclaim, render reads zimmer without synthesising, ffmpeg is spawned
//! only by render's command builder, and one module of providers names an
//! HTTP library. Comment lines are not read, so a doc can say what a crate
//! never does.
//!
//! A word matches only as a whole: `std::fs` does not match `std::fsx`, and
//! `print!` does not match inside `eprint!`. A grouped import (`use
//! std::{fs, io}`) is not seen; the crate docs and review still hold that
//! spelling, and the plain path is the one written here.

use std::path::Path;

use super::super::Finding;
use super::super::allow::{allowed, is_comment};

/// One boundary over source: words that may not be written under a path.
#[derive(Debug, Clone, Copy)]
pub struct Fence {
    /// The rule's name, as an `allow` names it.
    pub rule: &'static str,
    /// What the boundary is, and where it is written.
    pub says: &'static str,
    /// The path prefix the fence covers, relative to the repository root.
    pub inside: &'static str,
    /// Paths under it the words are allowed in: the one place that may.
    pub except: &'static [&'static str],
    /// The words that may not be written.
    pub words: &'static [&'static str],
    /// A word followed directly by one of these is allowed after all.
    pub unless_then: &'static [&'static str],
}

/// Every source fence.
pub const FENCES: &[Fence] = &[
    Fence {
        rule: "no-io",
        says: "zimmer does no I/O of any kind (crates/zimmer/src/lib.rs, *Boundary*; rusty \
               depends on it)",
        inside: "crates/zimmer/src/",
        except: &[],
        words: &[
            "std::fs",
            "std::net",
            "std::process",
            "std::env",
            "println!",
            "eprintln!",
            "print!",
            "eprint!",
        ],
        unless_then: &[],
    },
    Fence {
        rule: "no-io",
        says: "guide performs no I/O: it is text and arithmetic over text \
               (crates/guide/src/lib.rs, *Boundary*)",
        inside: "crates/guide/src/",
        except: &[],
        words: &["std::fs", "std::net", "std::process", "std::env"],
        unless_then: &[],
    },
    Fence {
        rule: "no-io",
        says: "the compositor reads no files and runs no programs (crates/compositor/src/lib.rs)",
        inside: "crates/compositor/src/",
        except: &[],
        words: &["std::fs", "std::process", "std::net"],
        unless_then: &[],
    },
    Fence {
        rule: "no-io",
        says: "core runs no programs and opens no sockets (crates/core/src/lib.rs)",
        inside: "crates/core/src/",
        except: &[],
        words: &["process::Command", "std::net"],
        unless_then: &[],
    },
    Fence {
        rule: "zimmer-reads",
        says: "render reads zimmer for loudness and songs only, never synthesising \
               (crates/render/src/lib.rs)",
        inside: "crates/render/",
        except: &[],
        words: &["scorsese_zimmer::"],
        unless_then: &["level", "survey"],
    },
    Fence {
        rule: "ffmpeg",
        says: "ffmpeg and ffprobe are spawned only by render's command builder \
               (`scorsese_render::tools::Tools`)",
        inside: "",
        except: &["crates/render/src/tools.rs"],
        words: &["Command::new(\"ffmpeg\"", "Command::new(\"ffprobe\""],
        unless_then: &[],
    },
    Fence {
        rule: "owner",
        says: "exactly one module of providers names an HTTP library \
               (crates/providers/src/api/mod.rs)",
        inside: "crates/providers/",
        except: &["crates/providers/src/api/http/"],
        words: &["ureq"],
        unless_then: &[],
    },
];

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The first of `fence`'s words standing whole on `line`, if any.
pub fn word_in(fence: &Fence, line: &str) -> Option<&'static str> {
    fence.words.iter().copied().find(|word| {
        line.match_indices(word).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let rest = &line[at + word.len()..];
            let cut_short = word.starts_with(is_word) && before.is_some_and(is_word)
                || word.ends_with(is_word) && rest.starts_with(is_word);
            !cut_short
                && !fence.unless_then.iter().any(|then| {
                    rest.strip_prefix(then)
                        .is_some_and(|after| !after.starts_with(is_word))
                })
        })
    })
}

/// Whether `fence` covers the file at `path`.
fn covers(fence: &Fence, path: &str) -> bool {
    path.starts_with(fence.inside) && !fence.except.iter().any(|except| path.starts_with(except))
}

/// Every fenced word in the code file at `path`, whose text is `text`.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    let shown = path.to_string_lossy().replace('\\', "/");
    let fences: Vec<&Fence> = FENCES
        .iter()
        .filter(|fence| covers(fence, &shown))
        .collect();
    if fences.is_empty() {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if is_comment(line) {
            continue;
        }
        for fence in &fences {
            let Some(word) = word_in(fence, line) else {
                continue;
            };
            if !allowed(&lines, index, fence.rule) {
                found.push(Finding {
                    path: path.to_path_buf(),
                    line: Some(index + 1),
                    rule: fence.rule,
                    message: format!("`{word}`: {}", fence.says),
                });
            }
        }
    }
    found
}
