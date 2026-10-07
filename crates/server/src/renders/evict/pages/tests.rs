use std::os::unix::fs::symlink;

use super::*;
use crate::renders::Quota;

/// A render cache under the temp folder, emptied.
fn cache(name: &str) -> RenderCache {
    let root = std::env::temp_dir().join(format!("scorsese-pages-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    RenderCache::new(root, Quota::bytes(u64::MAX))
}

/// A slot in user 7's project `project`, its frames and record last used
/// `hours` ago; the folder.
fn slot(cache: &RenderCache, project: i64, name: &str, hours: u64) -> PathBuf {
    let folder = cache.captures().root().join(format!("pages/7/{project}"));
    let slot = folder.join(SLOTS).join(name);
    std::fs::create_dir_all(&slot).unwrap();
    let at = SystemTime::now() - Duration::from_secs(hours * 60 * 60);
    for (file, bytes) in [(FRAMES, 100), (RECORD, 10)] {
        std::fs::write(slot.join(file), vec![0; bytes]).unwrap();
        let file = std::fs::File::options().append(true).open(slot.join(file));
        file.unwrap().set_modified(at).unwrap();
    }
    folder
}

fn fonts(folder: &Path) {
    std::fs::create_dir_all(folder.join("pages/fonts")).unwrap();
    std::fs::write(folder.join("pages/fonts/face.woff2"), [0; 5]).unwrap();
}

#[test]
fn an_idle_slot_goes_and_a_used_one_stays() {
    let cache = cache("idle");
    let folder = slot(&cache, 9, "old", 49);
    slot(&cache, 9, "new", 47);
    fonts(&folder);

    let evicted = idle(&folder);

    assert_eq!((evicted.captures, evicted.bytes), (1, 110));
    assert!(!folder.join("pages/old").exists());
    assert!(folder.join("pages/new/frames.mkv").is_file());
    assert!(folder.join("pages/fonts/face.woff2").is_file());
}

#[test]
fn a_project_left_with_no_slot_loses_its_folder_and_an_empty_user_too() {
    let cache = cache("emptied");
    let folder = slot(&cache, 9, "old", 49);
    fonts(&folder);

    let evicted = idle(&folder);
    tidy(&folder);

    assert_eq!((evicted.captures, evicted.bytes), (1, 115));
    assert!(!folder.exists());
    assert!(
        !folder.parent().unwrap().exists(),
        "user 7 had nothing else"
    );
}

#[test]
fn a_partial_goes_whatever_its_age_and_a_slot_with_only_one_goes_too() {
    let cache = cache("partial");
    let folder = slot(&cache, 9, "used", 1);
    std::fs::write(folder.join("pages/used/frames.41.partial.mkv"), [0; 7]).unwrap();
    std::fs::create_dir_all(folder.join("pages/killed")).unwrap();
    std::fs::write(folder.join("pages/killed/frames.42.partial.mkv"), [0; 3]).unwrap();

    let evicted = idle(&folder);

    assert_eq!((evicted.captures, evicted.bytes), (1, 10));
    assert!(!folder.join("pages/used/frames.41.partial.mkv").exists());
    assert!(folder.join("pages/used/frames.mkv").is_file());
    assert!(!folder.join("pages/killed").exists(), "it never finished");
}

#[test]
fn a_folder_is_held_by_a_pin_or_a_jobs_link_and_otherwise_not() {
    let cache = cache("held");
    let linked = slot(&cache, 1, "a", 49);
    let pinned = slot(&cache, 2, "a", 49);
    let free = slot(&cache, 3, "a", 49);
    let job = cache.captures().job(5).join(crate::captures::PROJECT);
    std::fs::create_dir_all(&job).unwrap();
    symlink(std::path::absolute(&linked).unwrap(), job.join(CACHE_DIR)).unwrap();
    let _pin = cache.pin(pinned.strip_prefix(&cache.root).unwrap());

    let held = held(&cache);
    let found: Vec<(i64, i64, bool)> = {
        let mut found: Vec<_> = projects(&cache)
            .into_iter()
            .map(|(user, project, folder)| (user, project, held.contains(&folder)))
            .collect();
        found.sort_unstable();
        found
    };

    assert_eq!(found, [(7, 1, true), (7, 2, true), (7, 3, false)]);
    assert_eq!(total(&cache), 330);
    assert!(free.exists());
}
