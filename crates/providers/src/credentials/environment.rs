//! The environment half of the resolution order — including the `.env` a
//! developer keeps at the root of a checkout.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The file a checkout keeps its keys in, gitignored, documented by
/// `.env.example` beside it.
const DOT_ENV: &str = ".env";

/// What the process was started with, and what the checkout adds to it.
///
/// Built rather than read on demand so that resolution is a pure function of a
/// value: the process environment is global mutable state shared with every
/// thread, and a test that has to set it cannot run beside another one.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    vars: BTreeMap<String, String>,
    dotenv: Option<PathBuf>,
}

impl Environment {
    /// Reads the process environment, then fills gaps from `.env` if there is
    /// one in `from` or any directory above it.
    ///
    /// **The process environment wins.** A `.env` is a convenience for a
    /// checkout, and an exported variable is somebody being deliberate right
    /// now — so the file fills gaps and never overrides.
    pub fn discover(from: &Path) -> Self {
        let mut environment = Self {
            vars: std::env::vars().collect(),
            dotenv: None,
        };
        if let Some(path) = find_dotenv(from) {
            for (key, value) in parse(&fs::read_to_string(&path).unwrap_or_default()) {
                environment.vars.entry(key).or_insert(value);
            }
            environment.dotenv = Some(path);
        }
        environment
    }

    /// An environment made of exactly these pairs, for tests and for callers
    /// that would rather be explicit than ambient.
    pub fn of<K: Into<String>, V: Into<String>>(pairs: impl IntoIterator<Item = (K, V)>) -> Self {
        Self {
            vars: pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            dotenv: None,
        }
    }

    /// The value of a variable, if it has one that is not blank.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars
            .get(name)
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// The `.env` this environment drew on, if it found one. Reported when a
    /// key is missing, so "where did it look" has a complete answer.
    pub fn dotenv(&self) -> Option<&Path> {
        self.dotenv.as_deref()
    }
}

/// The nearest `.env` at or above `from`.
///
/// Walking up rather than looking only in the working directory, because a
/// command is as often run from a project directory as from the checkout root,
/// and a key that resolves from one shell and not another is the confusing
/// half of this whole arrangement.
fn find_dotenv(from: &Path) -> Option<PathBuf> {
    from.ancestors()
        .map(|directory| directory.join(DOT_ENV))
        .find(|candidate| candidate.is_file())
}

/// The pairs in a `.env`, ignoring what such files conventionally allow.
///
/// Blank lines, `#` comments, an `export` prefix, and surrounding quotes.
/// Nothing more — no interpolation, no multi-line values, no escapes. This
/// reads a list of keys, and every feature past that is a shell being
/// reimplemented badly.
fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            let key = key.trim().strip_prefix("export ").unwrap_or(key).trim();
            (key.to_owned(), unquote(value.trim()).to_owned())
        })
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .collect()
}

/// A value with one matched pair of surrounding quotes removed.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|v| v.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}
