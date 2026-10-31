use std::path::Path;

use serde_json::json;

use crate::migrate::folder;
use crate::project::PROJECT_FILE_NAME;

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn put(root: &Path, relative: &str, bytes: &[u8]) {
    let file = root.join(relative);
    std::fs::create_dir_all(file.parent().expect("a parent")).unwrap();
    std::fs::write(file, bytes).unwrap();
}

fn read(root: &Path, relative: &str) -> Option<Vec<u8>> {
    std::fs::read(root.join(relative)).ok()
}

/// A brief's file name past its id: the hash in lowercase hex, then `.` and
/// anything but a folder.
#[test]
fn a_brief_s_name_is_a_hash_then_an_extension() {
    use super::is_brief_name;
    assert!(is_brief_name(&format!("{DIGEST}.mp3.words.json")));
    assert!(!is_brief_name(DIGEST), "no extension");
    assert!(
        !is_brief_name(&format!("{DIGEST}x.jpg")),
        "no dot after the hash"
    );
    assert!(
        !is_brief_name(&format!("{}.jpg", "g".repeat(64))),
        "not hex"
    );
    assert!(
        !is_brief_name(&format!("{}.jpg", DIGEST.to_uppercase())),
        "not lowercase"
    );
    assert!(!is_brief_name(&format!("{DIGEST}.d/x.jpg")), "a folder");
}

/// A local folder whose paid files are named for ids the migration renames:
/// each is linked at the name its new id gives it, so the next run finds its
/// brief drawn and pays nothing. The file paid for stays where it was, byte
/// for byte; the asset's path follows to the new name.
#[test]
fn a_renamed_asset_s_paid_files_are_found_under_its_new_name() {
    let dir = crate::scratch::fresh("scorsese-migrate-carry", ".scor");
    let still = format!("generated/shot one-{DIGEST}.jpg");
    let nested = format!("generated/a/b-{DIGEST}.mp3");
    put(&dir, &still, b"paid still");
    put(&dir, &nested, b"paid line");
    put(&dir, &format!("{nested}.words.json"), b"timings");
    put(&dir, "generated/shot one-draft.jpg", b"not a brief's");
    let document = json!({
        "schema_version": 55, "name": "Carried", "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "shot one", "kind": "generated_image", "state": "generated",
              "prompt": "a poster", "path": still, "image": {} },
            { "id": "a/b", "kind": "image", "path": "assets/b.png" }
        ],
        "tracks": []
    });
    std::fs::write(dir.join(PROJECT_FILE_NAME), document.to_string()).unwrap();

    let done = folder(&dir).expect("migrates").expect("it was behind");
    assert_eq!(done.from, 55);
    let renamed: Vec<(&str, &str)> = (done.renamed.iter())
        .map(|(old, new)| (old.as_str(), new.as_str()))
        .collect();
    assert_eq!(renamed, [("shot one", "shot-one"), ("a/b", "a-b")]);

    let carried = format!("generated/shot-one-{DIGEST}.jpg");
    assert_eq!(read(&dir, &carried).as_deref(), Some(&b"paid still"[..]));
    assert_eq!(read(&dir, &still).as_deref(), Some(&b"paid still"[..]));
    let line = format!("generated/a-b-{DIGEST}.mp3");
    assert_eq!(read(&dir, &line).as_deref(), Some(&b"paid line"[..]));
    let words = format!("{line}.words.json");
    assert_eq!(read(&dir, &words).as_deref(), Some(&b"timings"[..]));
    assert_eq!(
        read(&dir, "generated/shot-one-draft.jpg"),
        None,
        "no brief's file"
    );
    let project = crate::Project::load(&dir).expect("the folder loads");
    assert_eq!(project.assets[0].id.as_str(), "shot-one");
    assert_eq!(
        project.assets[0].path.as_ref().map(|p| p.as_str()),
        Some(&*carried),
        "the asset plays its file under the name its brief now gives it"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A file already at the new name is somebody's: it is never replaced.
#[test]
fn a_file_already_at_the_new_name_is_left_as_it_was() {
    let dir = crate::scratch::fresh("scorsese-migrate-carry-kept", ".scor");
    put(&dir, &format!("generated/x y-{DIGEST}.jpg"), b"old");
    put(
        &dir,
        &format!("generated/x-y-{DIGEST}.jpg"),
        b"there already",
    );
    let document = json!({
        "schema_version": 55, "name": "Kept", "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "x y", "kind": "image", "path": "assets/x.png" }],
        "tracks": []
    });
    std::fs::write(dir.join(PROJECT_FILE_NAME), document.to_string()).unwrap();

    folder(&dir).expect("migrates");

    let kept = read(&dir, &format!("generated/x-y-{DIGEST}.jpg"));
    assert_eq!(kept.as_deref(), Some(&b"there already"[..]));
    std::fs::remove_dir_all(&dir).ok();
}
