//! Which browser a capture runs, in one order (#776):
//!
//! 1. [`CHROME_ENV`] — somebody named one, and that one is used, working or
//!    not: falling past a broken override to another build would draw the page
//!    with pixels nobody asked for.
//! 2. The build this program downloaded ([`super::supply`]).
//! 3. `chrome-headless-shell` on `PATH`.
//! 4. Downloading the build (2), the first time a page needs drawing — so a
//!    person never has to know a browser is involved.

use std::ffi::OsString;
use std::path::Path;

use super::browser::{CHROME_ENV, Chrome, ChromeError};
use super::supply::{self, Supply};

/// The name looked for on `PATH`.
const ON_PATH: &str = "chrome-headless-shell";

/// A browser found, and whether finding it took a download.
#[derive(Debug)]
pub(crate) struct Found {
    pub(crate) chrome: Chrome,
    pub(crate) fetched: bool,
}

/// The browser, by the order above.
pub(crate) fn find() -> Result<Found, ChromeError> {
    find_in(
        std::env::var_os(CHROME_ENV),
        supply::supplied(),
        Path::new(ON_PATH),
    )
}

fn find_in(
    named: Option<OsString>,
    supply: Option<&Supply>,
    on_path: &Path,
) -> Result<Found, ChromeError> {
    let found = |chrome| Found {
        chrome,
        fetched: false,
    };
    if let Some(binary) = named {
        return Chrome::at(binary).map(found);
    }
    if let Some(binary) = supply.and_then(Supply::installed) {
        return Chrome::at(binary).map(found);
    }
    let missing = match Chrome::at(on_path) {
        Ok(chrome) => return Ok(found(chrome)),
        Err(missing) => missing,
    };
    let Some(supply) = supply else {
        return Err(missing);
    };
    let binary = supply.fetch().map_err(ChromeError::Fetch)?;
    Chrome::at(binary).map(|chrome| Found {
        chrome,
        fetched: true,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A stand-in browser that answers `--version` with `version`.
    fn browser(dir: &Path, version: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(format!("chrome-{version}"));
        std::fs::write(&path, format!("#!/bin/sh\necho 'Chrome {version}'\n")).expect("script");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        path
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scorsese-find-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    /// A supply that has `installed` (or nothing) and fetches `fetched`,
    /// counting the fetches.
    fn supply(
        installed: Option<PathBuf>,
        fetched: Result<PathBuf, String>,
    ) -> (Supply, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&count);
        let supply = Supply::new(
            move || installed.clone(),
            move || {
                counted.fetch_add(1, Ordering::Relaxed);
                fetched.clone()
            },
        );
        (supply, count)
    }

    const NOWHERE: &str = "/nonexistent/chrome-headless-shell";

    #[test]
    fn the_environment_wins_then_the_download_then_path() {
        let dir = scratch("order");
        let (named, installed, path) = (
            browser(&dir, "1.0"),
            browser(&dir, "2.0"),
            browser(&dir, "3.0"),
        );
        let (with_install, fetches) = supply(Some(installed), Err("no".to_owned()));
        let (without, _) = supply(None, Err("no".to_owned()));
        let version = |found: Found| (found.chrome.version().to_owned(), found.fetched);

        let got = find_in(Some(named.into()), Some(&with_install), &path).expect("found");
        assert_eq!(version(got), ("1.0".to_owned(), false));
        let got = find_in(None, Some(&with_install), &path).expect("found");
        assert_eq!(version(got), ("2.0".to_owned(), false));
        let got = find_in(None, Some(&without), &path).expect("found");
        assert_eq!(version(got), ("3.0".to_owned(), false));
        assert_eq!(fetches.load(Ordering::Relaxed), 0, "nothing was downloaded");
    }

    #[test]
    fn nothing_found_is_downloaded_and_said_so() {
        let dir = scratch("fetch");
        let (fetching, fetches) = supply(None, Ok(browser(&dir, "4.0")));
        let found = find_in(None, Some(&fetching), Path::new(NOWHERE)).expect("fetched");
        assert_eq!((found.chrome.version(), found.fetched), ("4.0", true));
        assert_eq!(fetches.load(Ordering::Relaxed), 1);

        let (offline, _) = supply(None, Err("offline".to_owned()));
        let failed = find_in(None, Some(&offline), Path::new(NOWHERE));
        assert!(matches!(failed, Err(ChromeError::Fetch(why)) if why == "offline"));
    }

    #[test]
    fn a_broken_override_is_reported_rather_than_passed_over() {
        let dir = scratch("broken");
        let (fetching, fetches) = supply(Some(browser(&dir, "2.0")), Ok(browser(&dir, "4.0")));
        let refused = find_in(Some(NOWHERE.into()), Some(&fetching), Path::new(NOWHERE));
        assert!(
            matches!(refused, Err(ChromeError::NotFound { .. })),
            "{refused:?}"
        );
        assert_eq!(fetches.load(Ordering::Relaxed), 0);
        let alone = find_in(None, None, Path::new(NOWHERE));
        assert!(
            matches!(alone, Err(ChromeError::NotFound { .. })),
            "{alone:?}"
        );
    }
}
