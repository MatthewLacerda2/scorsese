//! A stored project laid out as a `.scor` folder for a render: the document
//! written, every file linked by hash, and nothing left behind or reached
//! outside the folder.

use std::path::{Path, PathBuf};

use scorsese_core::{
    Asset, AssetHealth, AssetId, AssetKind, Fps, HashCheck, Project, ProjectPath, asset_status,
    hash_bytes,
};
use scorsese_server::projects::media::{self, MaterialiseError, materialise};

/// A fresh directory for one test, standing in for a user's storage.
fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "scorsese-server-materialise-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A user's file with these bytes, stored under its hash; the hash.
fn store(library: &Path, bytes: &[u8]) -> String {
    let hash = hash_bytes(bytes);
    std::fs::write(library.join(&hash), bytes).unwrap();
    hash
}

fn asset(id: &str, path: ProjectPath, hash: &str) -> Asset {
    let mut asset = Asset::imported(AssetId::new(id), AssetKind::Video, path);
    asset.sha256 = Some(hash.to_owned());
    asset
}

#[test]
fn every_file_is_linked_by_hash_and_the_folder_goes_when_dropped() {
    let library = scratch("library");
    let footage = store(&library, b"footage");
    let rendered = store(&library, b"a generated shot");
    let mut project = Project::new("teaser", Fps::default());
    project.assets = vec![
        asset(
            "footage",
            media::library_path(&footage, Some("mp4")),
            &footage,
        ),
        asset(
            "shot",
            ProjectPath::new(format!("generated/{rendered}.mp4")),
            &rendered,
        ),
        asset(
            "lost",
            media::library_path(&"0".repeat(64), Some("mp4")),
            &"0".repeat(64),
        ),
    ];
    let locate = |hash: &str| Some(library.join(hash)).filter(|path| path.exists());
    let at = scratch("folder").join("render.scor");

    let folder = materialise(&project, &at, &locate).unwrap();
    let laid_out = Project::load(folder.root()).expect("the folder is a project");
    assert_eq!(laid_out.assets, project.assets);
    let health: Vec<AssetHealth> = asset_status(&laid_out, folder.root(), HashCheck::Verify)
        .into_iter()
        .map(|status| status.health)
        .collect();
    // Unprobed: present and hashing true, with nothing probed about them.
    assert_eq!(
        health,
        [
            AssetHealth::Unprobed,
            AssetHealth::Unprobed,
            AssetHealth::Missing
        ]
    );
    assert_eq!(folder.missing(), [AssetId::new("lost")]);

    drop(folder);
    assert!(!at.exists(), "the folder is removed");
    assert_eq!(
        std::fs::read(library.join(&footage)).unwrap(),
        b"footage",
        "not what it linked"
    );
}

#[test]
fn a_path_outside_the_project_is_refused_and_leaves_nothing() {
    let library = scratch("escape-library");
    let hash = store(&library, b"bytes");
    let at = scratch("escape").join("render.scor");
    for bad in ["../outside.mp4", "/etc/passwd", "assets/../../x"] {
        let mut project = Project::new("p", Fps::default());
        project.assets = vec![asset("bad", ProjectPath::new(bad), &hash)];
        let outcome = materialise(&project, &at, &|_: &str| Some(library.join(&hash)));
        assert!(
            matches!(outcome, Err(MaterialiseError::BadPath { .. })),
            "{bad}: {outcome:?}"
        );
        assert!(!at.exists(), "{bad}: the folder was cleaned up");
    }
}

#[test]
fn the_document_cannot_be_written_through_a_link() {
    // An asset claiming `project.json` as its path gets no link: the document
    // is already there, and the user's file is never written to.
    let library = scratch("clobber-library");
    let hash = store(&library, b"precious");
    let mut project = Project::new("p", Fps::default());
    project.assets = vec![asset("sly", ProjectPath::new("project.json"), &hash)];
    let at = scratch("clobber").join("render.scor");

    let folder = materialise(&project, &at, &|_: &str| Some(library.join(&hash))).unwrap();
    assert!(!folder.root().join("project.json").is_symlink());
    assert_eq!(std::fs::read(library.join(&hash)).unwrap(), b"precious");

    let again = materialise(&project, &at, &|_: &str| None);
    assert!(
        matches!(again, Err(MaterialiseError::Exists(_))),
        "{again:?}"
    );
    assert!(
        at.exists(),
        "a refusal never removes what was already there"
    );
}
