//! Where the browser comes from when nothing in the environment names one: the
//! build this machine downloaded, and how to download it (#776).
//!
//! This crate reaches no network and keeps nothing per machine, so it is
//! **told** both, once, by the program it runs in — [`supply`]. The CLI, the
//! MCP server and the desktop app tell it (`scorsese_providers::chromium` is
//! what they hand over); the hosted server does not, so a render there finds
//! its browser in the environment or not at all (#778), and never fetches one.
//!
//! Told per process rather than per [`crate::Renderer`] because that is the
//! scope of the answer: *may this program download a browser* is a fact about
//! the program, and the MCP tools the hosted server also runs would otherwise
//! each have to be told it, and could each be told wrong.

use std::path::PathBuf;
use std::sync::OnceLock;

/// The pinned build on this machine, and how to fetch it.
pub struct Supply {
    installed: Box<dyn Fn() -> Option<PathBuf> + Send + Sync>,
    fetch: Box<dyn Fn() -> Result<PathBuf, String> + Send + Sync>,
}

impl Supply {
    /// `installed` answers the build already downloaded, touching nothing but
    /// the disk; `fetch` downloads it and answers its binary, and is called
    /// only when a page needs drawing and no browser was found any other way.
    pub fn new(
        installed: impl Fn() -> Option<PathBuf> + Send + Sync + 'static,
        fetch: impl Fn() -> Result<PathBuf, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            installed: Box::new(installed),
            fetch: Box::new(fetch),
        }
    }

    pub(super) fn installed(&self) -> Option<PathBuf> {
        (self.installed)()
    }

    pub(super) fn fetch(&self) -> Result<PathBuf, String> {
        (self.fetch)()
    }
}

impl std::fmt::Debug for Supply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Supply")
    }
}

static SUPPLIED: OnceLock<Supply> = OnceLock::new();

/// Tells this program's renders where the downloaded browser is and how to
/// fetch it. Once per process; a second call changes nothing and answers
/// `false`.
pub fn supply(supply: Supply) -> bool {
    SUPPLIED.set(supply).is_ok()
}

/// What [`supply`] was told, if anything.
pub(super) fn supplied() -> Option<&'static Supply> {
    SUPPLIED.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Told once per process, and the first telling stands. Nothing else in
    /// this crate's unit tests looks for a browser, so setting it here moves
    /// no other test.
    #[test]
    fn a_program_tells_it_once() {
        let failing = || Supply::new(|| None, || Err("offline".to_owned()));
        assert_eq!(format!("{:?}", failing()), "Supply");
        let first = supply(failing());
        assert!(first, "nothing else in these tests supplies one");
        assert!(!supply(failing()), "a second telling changes nothing");
        let told = supplied().expect("told");
        assert_eq!(told.installed(), None);
        assert_eq!(told.fetch(), Err("offline".to_owned()));
    }
}
