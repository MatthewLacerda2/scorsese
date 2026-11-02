use std::os::unix::fs::symlink;

use super::*;
use crate::renders::Quota;

/// A render cache under the temp folder, emptied.
fn cache(name: &str) -> RenderCache {
    let root = crate::scratch::fresh(&format!("scorsese-pages-{name}"), "");
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
    let used = folder.join("pages/used/part-0-31.41-0.capturing");
    std::fs::create_dir_all(&used).unwrap();
    std::fs::write(used.join("frames.mkv"), [0; 7]).unwrap();
    let killed = folder.join("pages/killed/part-0-31.42-0.capturing");
    std::fs::create_dir_all(&killed).unwrap();
    std::fs::write(killed.join("frames.mkv"), [0; 3]).unwrap();

    let evicted = idle(&folder);

    assert_eq!((evicted.captures, evicted.bytes), (1, 10));
    assert!(!used.exists());
    assert!(folder.join("pages/used/frames.mkv").is_file());
    assert!(!folder.join("pages/killed").exists(), "it never finished");
}

#[test]
fn a_project_gone_counts_its_slots_and_every_byte() {
    let cache = cache("gone");
    let folder = slot(&cache, 9, "a", 1);
    fonts(&folder);

    let evicted = gone(&folder);

    assert_eq!((evicted.captures, evicted.bytes), (1, 115));
    assert!(!folder.exists());
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

#[test]
fn a_slot_of_pieces_ages_by_its_newest_on_any_shelf_and_goes_whole() {
    let cache = cache("pieces");
    let folder = cache.captures().root().join("pages/7/9");
    let aged = |slot: &str, file: &str, hours: u64| {
        let path = folder.join(SLOTS).join(slot).join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, [0; 4]).unwrap();
        let at = SystemTime::now() - Duration::from_secs(hours * 60 * 60);
        let file = std::fs::File::options().append(true).open(&path).unwrap();
        file.set_modified(at).unwrap();
    };
    aged("old", "told-none/part-0-30.mkv", 49);
    aged("old", "told-none/part-0-30.json", 49);
    aged("used", "told-none/part-0-30.mkv", 49);
    aged("used", "told-1a2b/part-60-90.mkv", 1);
    aged("used", "told-1a2b/part-60-90.json", 1);
    aged("used", "part-90-120.7-0.capturing/frames.mkv", 0);

    let evicted = idle(&folder);

    assert_eq!((evicted.captures, evicted.bytes), (1, 12));
    assert!(!folder.join("pages/old").exists());
    assert!(
        folder.join("pages/used/told-none/part-0-30.mkv").is_file(),
        "whole or not at all"
    );
    assert!(!folder.join("pages/used/part-90-120.7-0.capturing").exists());
}

#[test]
fn a_link_in_the_page_cache_goes_itself_and_its_target_survives() {
    let cache = cache("links");
    let outside = cache.root.join("outside");
    // A folder outside the cache shaped like idle slots, which a sweep that
    // followed a link to it would delete.
    let victim = |name: &str| {
        let dir = outside.join(name);
        let at = SystemTime::now() - Duration::from_secs(49 * 60 * 60);
        for file in ["told-none/frames.mkv", "old/frames.mkv"] {
            std::fs::create_dir_all(dir.join(file).parent().unwrap()).unwrap();
            std::fs::write(dir.join(file), [0; 3]).unwrap();
            let file = std::fs::File::options().append(true).open(dir.join(file));
            file.unwrap().set_modified(at).unwrap();
        }
        dir
    };
    // A slot that is a link to a folder outside the cache.
    let folder = slot(&cache, 9, "used", 1);
    let slot_target = victim("slot");
    symlink(&slot_target, folder.join("pages/linked")).unwrap();
    let evicted = idle(&folder);
    assert!(
        std::fs::symlink_metadata(folder.join("pages/linked")).is_err(),
        "the link itself goes"
    );
    assert!(slot_target.join("told-none/frames.mkv").is_file());
    assert!(slot_target.join("old/frames.mkv").is_file());
    assert!(folder.join("pages/used/frames.mkv").is_file());
    assert_eq!(evicted.captures, 0, "a link is no capture");

    // A `pages/` that is a link: nothing behind it is a slot of this project.
    let folder = cache.captures().root().join("pages/7/10");
    std::fs::create_dir_all(&folder).unwrap();
    let pages_target = victim("pages");
    symlink(&pages_target, folder.join(SLOTS)).unwrap();
    idle(&folder);
    assert!(pages_target.join("old/frames.mkv").is_file());
    assert!(pages_target.join("told-none/frames.mkv").is_file());
    assert!(!folder.exists(), "the project held no slot, so it goes");

    // A project deleted with its `pages/` a link counts nothing behind it.
    let folder = cache.captures().root().join("pages/7/11");
    std::fs::create_dir_all(&folder).unwrap();
    symlink(&pages_target, folder.join(SLOTS)).unwrap();
    assert_eq!(gone(&folder).captures, 0);
    assert!(pages_target.join("old/frames.mkv").is_file());
}
