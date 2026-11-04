//! A local project's proxies folder is made from its root (#1718): a `.scor`
//! received from someone else may carry `cache` itself as a link, and no
//! proxy is made where it leads.

#![cfg(unix)]

use scorsese_render::preview;

use super::common::scratch;

#[test]
fn the_proxies_folder_is_made_in_the_project() {
    let dir = scratch::dir("proxy-folder");
    let made = preview::made_folder(&dir).expect("made");
    assert_eq!(made, preview::folder(&dir));
    assert!(made.is_dir());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_linked_cache_folder_is_refused() {
    let dir = scratch::dir("proxy-folder-linked");
    let outside = scratch::dir("proxy-folder-linked-outside");
    let _ = std::fs::remove_dir_all(dir.join("cache"));
    std::os::unix::fs::symlink(&outside, dir.join("cache")).expect("a link");

    let refused = preview::made_folder(&dir).expect_err("made through a link");

    assert!(refused.to_string().contains("link"), "{refused}");
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&outside).ok();
}
