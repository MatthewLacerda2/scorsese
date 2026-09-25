//! Tokens for a project on this machine: a file each, under `cache/`.
//!
//! `cache/` because that is what a token is — rebuildable. Losing one costs a
//! quote, which is free, and nothing else; a directory that is gitignored and
//! never copied between machines is exactly where something that should not
//! outlive fifteen minutes belongs.
//!
//! In the project rather than in the server's memory, because the MCP server
//! holds no state between calls, and a server restarted by a `git pull` between
//! the quote and the yes should not turn a yes into a refusal.

use std::io;
use std::path::{Path, PathBuf};

use scorsese_core::CACHE_DIR;

use super::token::{Issued, Quotes, well_formed};

/// Where a project's live tokens are kept, inside `cache/`.
pub const QUOTES_DIR: &str = "quotes";

/// The tokens of one project.
#[derive(Debug, Clone)]
pub struct ProjectQuotes {
    dir: PathBuf,
}

impl ProjectQuotes {
    /// The store for the project at `root`.
    pub fn new(root: &Path) -> Self {
        Self {
            dir: root.join(CACHE_DIR).join(QUOTES_DIR),
        }
    }

    /// Where one token's record lives. Only ever asked of a well-formed token,
    /// so the name cannot step outside the directory.
    fn file(&self, token: &str) -> PathBuf {
        self.dir.join(format!("{token}.json"))
    }

    /// Removes every record that expired before `now`.
    ///
    /// Best-effort and on every issue, so the directory holds the handful of
    /// quotes somebody is actually deciding on rather than every one ever made.
    /// A record that cannot be read is left alone: it will be refused if it is
    /// ever handed in, and deleting what cannot be understood is not tidying.
    fn sweep(&self, now: i64) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            let expired = std::fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Issued>(&bytes).ok())
                .is_some_and(|issued| issued.expires_at < now);
            if expired {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

impl Quotes for ProjectQuotes {
    type Error = io::Error;

    fn put(&self, issued: &Issued) -> io::Result<()> {
        self.sweep(issued.issued_at);
        std::fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_vec_pretty(issued).map_err(io::Error::other)?;
        scorsese_core::write::atomically(&self.file(&issued.token), json)
    }

    fn take(&self, token: &str) -> io::Result<Option<Issued>> {
        if !well_formed(token) {
            return Ok(None);
        }
        let path = self.file(token);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        // Whoever removes the file is whoever spends the token. Two callers
        // can both have read it; only one of them gets to delete it.
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}
