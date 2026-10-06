//! The download's local half, on a zip the test writes: nothing here touches a
//! network (CLAUDE.md), and everything that can go wrong after the bytes
//! arrive — a wrong checksum, a zip with no browser, a stamp from an older
//! pin, a second fetch — is decided by code these reach.

use std::cell::Cell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::*;

/// A fresh folder of the test's own.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scorsese-chromium-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

/// A zip shaped like Chrome for Testing's — `chrome-headless-shell-linux64/`
/// with an executable in it, unless `binary` is false — and the build that
/// names it, checksum included.
fn zip(dir: &Path, binary: bool) -> (PathBuf, Build) {
    let path = dir.join("upstream.zip");
    let mut writer = zip::ZipWriter::new(fs::File::create(&path).expect("zip file"));
    let executable = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    let plain = zip::write::SimpleFileOptions::default().unix_permissions(0o644);
    if binary {
        writer
            .start_file(
                "chrome-headless-shell-linux64/chrome-headless-shell",
                executable,
            )
            .expect("entry");
        writer
            .write_all(b"#!/bin/sh\necho 154.0.8037.92\n")
            .expect("bytes");
    }
    writer
        .start_file("chrome-headless-shell-linux64/icudtl.dat", plain)
        .expect("entry");
    writer.write_all(b"data").expect("bytes");
    writer.finish().expect("zip written");
    let build = Build {
        version: "154.0.8037.92".to_owned(),
        url: "https://example.invalid/chrome.zip".to_owned(),
        sha256: scorsese_core::pool::hash_file(&path).expect("hash"),
        platform: Platform::Linux64,
    };
    (path, build)
}

/// Serves one file, and counts how often it was asked.
struct Local {
    file: PathBuf,
    asked: Cell<usize>,
}

impl Local {
    fn new(file: PathBuf) -> Self {
        Self {
            file,
            asked: Cell::new(0),
        }
    }
}

impl Source for Local {
    fn get(
        &self,
        _url: &str,
        to: &mut dyn Write,
        progress: &mut dyn FnMut(Fetching),
    ) -> Result<(), ChromiumError> {
        self.asked.set(self.asked.get() + 1);
        let bytes = fs::read(&self.file)?;
        let total = Some(bytes.len() as u64);
        progress(Fetching { received: 0, total });
        to.write_all(&bytes)?;
        progress(Fetching {
            received: bytes.len() as u64,
            total,
        });
        Ok(())
    }
}

/// Nothing left behind but the install: no zip, no work folder.
fn leftovers(root: &Path, build: &Build) -> Vec<String> {
    fs::read_dir(root.join(&build.version))
        .expect("version folder")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| *name != build.platform.folder())
        .collect()
}

#[test]
fn a_fetch_installs_the_verified_build_and_a_second_finds_it() {
    let dir = scratch("installs");
    let (file, build) = zip(&dir, true);
    let root = dir.join("chromium");
    let source = Local::new(file);
    let mut heard = Vec::new();

    let binary = fetch_with(&source, &root, &build, &mut |at| heard.push(at)).expect("installed");
    assert_eq!(
        binary,
        root.join("154.0.8037.92/chrome-headless-shell-linux64/chrome-headless-shell")
    );
    assert_eq!(
        heard.first().map(|at| at.received),
        Some(0),
        "said before any byte"
    );
    assert_eq!(
        heard.last().map(|at| Some(at.received)),
        Some(heard[0].total)
    );
    assert_eq!(install::installed_in(&root, &build), Some(binary.clone()));
    assert!(
        leftovers(&root, &build).is_empty(),
        "{:?}",
        leftovers(&root, &build)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&binary).expect("binary").permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "still executable: {mode:o}");
    }

    let again = fetch_with(&source, &root, &build, &mut |_| {
        panic!("nothing to download")
    })
    .expect("found");
    assert_eq!((again, source.asked.get()), (binary, 1));
}

#[test]
fn a_zip_that_is_not_the_pinned_build_installs_nothing() {
    let dir = scratch("checksum");
    let (file, mut build) = zip(&dir, true);
    build.sha256 = "0".repeat(64);
    let root = dir.join("chromium");
    let refused = fetch_with(&Local::new(file), &root, &build, &mut |_| {});
    assert!(
        matches!(refused, Err(ChromiumError::Checksum { .. })),
        "{refused:?}"
    );
    assert_eq!(install::installed_in(&root, &build), None);
    assert!(
        leftovers(&root, &build).is_empty(),
        "{:?}",
        leftovers(&root, &build)
    );
}

#[test]
fn a_zip_with_no_browser_in_it_installs_nothing() {
    let dir = scratch("empty");
    let (file, build) = zip(&dir, false);
    let root = dir.join("chromium");
    let refused = fetch_with(&Local::new(file), &root, &build, &mut |_| {});
    assert!(
        matches!(refused, Err(ChromiumError::Unpack(_))),
        "{refused:?}"
    );
    assert!(
        !root
            .join(&build.version)
            .join(build.platform.folder())
            .exists()
    );
}

#[test]
fn an_install_stamped_by_another_pin_is_downloaded_again() {
    let dir = scratch("restamp");
    let (file, build) = zip(&dir, true);
    let root = dir.join("chromium");
    let source = Local::new(file);
    fetch_with(&source, &root, &build, &mut |_| {}).expect("installed");
    let stamp = root
        .join(&build.version)
        .join(build.platform.folder())
        .join(".sha256");
    fs::write(&stamp, "f".repeat(64)).expect("an older pin's stamp");
    assert_eq!(install::installed_in(&root, &build), None);

    fetch_with(&source, &root, &build, &mut |_| {}).expect("reinstalled");
    assert_eq!(source.asked.get(), 2);
    assert_eq!(fs::read_to_string(&stamp).expect("stamp"), build.sha256);
}

#[test]
fn a_download_that_fails_leaves_nothing_behind() {
    struct Offline;
    impl Source for Offline {
        fn get(
            &self,
            url: &str,
            _: &mut dyn Write,
            _: &mut dyn FnMut(Fetching),
        ) -> Result<(), ChromiumError> {
            Err(ChromiumError::Download(HttpError::Unreachable {
                url: url.to_owned(),
                message: "offline".to_owned(),
            }))
        }
    }
    let dir = scratch("offline");
    let (_, build) = zip(&dir, true);
    let root = dir.join("chromium");
    let failed = fetch_with(&Offline, &root, &build, &mut |_| {});
    assert!(
        matches!(failed, Err(ChromiumError::Download(_))),
        "{failed:?}"
    );
    assert!(
        leftovers(&root, &build).is_empty(),
        "{:?}",
        leftovers(&root, &build)
    );
}

#[test]
fn the_start_is_said_in_one_line_with_its_size_when_known() {
    let known = Fetching {
        received: 0,
        total: Some(99_221_129),
    };
    assert_eq!(
        known.starting(),
        "fetching the page renderer, once (100 MB) — it draws html clips"
    );
    let unknown = Fetching {
        received: 0,
        total: None,
    };
    assert_eq!(
        unknown.starting(),
        "fetching the page renderer, once — it draws html clips"
    );
}
