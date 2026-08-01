//! Importing a whole directory: what comes in, in what order, and what is
//! passed over rather than guessed at.

use std::path::{Path, PathBuf};

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{ASSETS_DIR, ImportError, Project, SkipReason, import_path};

/// A folder of two clips, a licence file nothing can infer a kind from, and a
/// subfolder with a third clip in it — the shape a folder of footage actually
/// arrives in.
fn footage(dir: &Path) -> PathBuf {
    source_file(dir, "wide.mp4", b"the wide shot");
    source_file(dir, "close.mp4", b"the close-up");
    source_file(dir, "LICENCE.txt", b"the font's licence");
    source_file(dir, "b-roll/extra.mp4", b"a shot one level down");
    dir.join("sources")
}

/// Every file now sitting in the project's `assets/`.
fn pool(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir.join(ASSETS_DIR))
        .expect("a project has an assets directory")
        .filter_map(|entry| Some(entry.ok()?.file_name().to_str()?.to_owned()))
        .collect();
    names.sort();
    names
}

#[test]
fn a_directory_imports_the_media_directly_inside_it_sorted_by_name() {
    let (dir, mut project) = new_project("dir-import");
    let folder = footage(&dir);

    let report = import_path(&mut project, &dir, &folder, None, &StubProbe::video())
        .expect("the folder imports");

    let ids: Vec<_> = report
        .imported
        .iter()
        .map(|one| one.id.as_str().to_owned())
        .collect();
    assert_eq!(ids, ["close", "wide"], "sorted by file name, every time");
    assert!(report.imported.iter().all(|one| !one.reused));
    assert_eq!(project.assets.len(), 2, "the folder itself is not an asset");
    assert_eq!(pool(&dir), ["close.mp4", "wide.mp4"]);
}

#[test]
fn what_is_not_media_is_skipped_and_named_rather_than_ignored() {
    // A licence beside the footage and a mistyped video look identical from
    // the outside unless the reply says which was which.
    let (dir, mut project) = new_project("dir-skips");
    let folder = footage(&dir);

    let report =
        import_path(&mut project, &dir, &folder, None, &StubProbe::video()).expect("imports");

    let skipped: Vec<_> = report
        .skipped
        .iter()
        .map(|one| (one.source.as_str(), one.why))
        .collect();
    assert_eq!(
        skipped,
        [
            ("LICENCE.txt", SkipReason::UnknownKind),
            ("b-roll", SkipReason::NotAFile),
        ]
    );
    assert!(
        !project.assets.iter().any(|a| a.id.as_str() == "extra"),
        "import does not recurse"
    );
}

#[test]
fn an_id_an_asset_already_answers_to_refuses_the_whole_folder() {
    let (dir, mut project) = new_project("dir-collision");
    let folder = footage(&dir);
    // Different bytes, the same name as one of the files in the folder.
    let earlier = source_file(&dir, "elsewhere/wide.mp4", b"an entirely different shot");
    import_path(&mut project, &dir, &earlier, None, &StubProbe::video()).expect("the first import");
    let before = pool(&dir);

    let refused = import_path(&mut project, &dir, &folder, None, &StubProbe::video())
        .expect_err("`wide` is taken");

    assert!(
        matches!(refused, ImportError::IdTaken { ref id, .. } if id.as_str() == "wide"),
        "got {refused}"
    );
    assert_eq!(project.assets.len(), 1, "nothing was added");
    assert_eq!(pool(&dir), before, "and nothing was copied");
}

#[test]
fn re_running_a_folder_import_copies_nothing_and_adds_nothing() {
    // The thing an agent does by accident. Content already in the pool is the
    // asset that holds it, not a collision.
    let (dir, mut project) = new_project("dir-again");
    let folder = footage(&dir);
    import_path(&mut project, &dir, &folder, None, &StubProbe::video()).expect("the first import");

    let again =
        import_path(&mut project, &dir, &folder, None, &StubProbe::video()).expect("and again");

    assert!(again.imported.iter().all(|one| one.reused));
    assert_eq!(project.assets.len(), 2);
    assert_eq!(pool(&dir), ["close.mp4", "wide.mp4"]);
}

#[test]
fn a_single_file_is_still_one_asset_and_no_skips() {
    let (dir, mut project) = new_project("dir-one-file");
    let source = source_file(&dir, "wide.mp4", b"the wide shot");

    let report = import_path(&mut project, &dir, &source, None, &StubProbe::video())
        .expect("one file imports");

    assert_eq!(report.imported.len(), 1);
    assert_eq!(report.imported[0].id.as_str(), "wide");
    assert!(report.skipped.is_empty());

    project.save(&dir).expect("save");
    assert_eq!(Project::load(&dir).expect("load validates").assets.len(), 1);
}
