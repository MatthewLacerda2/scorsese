//! A link waiting where a download writes before it lands — the fixed name it
//! wrote under on main, or the next fresh ones — is never written through
//! (#1672): a `.scor` received from someone else can carry one in its stock
//! cache.

#![cfg(unix)]

use std::os::unix::fs::symlink;

use scorsese_core::write::scratch_beside;
use scorsese_providers::stock::{cache_dir, labelled_previews, search};

use crate::common::{project, scratch};
use crate::stock::fake::{Fake, query};

#[test]
fn a_link_where_a_preview_is_written_is_left_as_it_was() {
    let (root, _) = project("stock-planted");
    let outside = scratch("stock-planted-outside");
    std::fs::create_dir_all(&outside).unwrap();
    let victim = outside.join("victim.txt");
    std::fs::write(&victim, "the victim's own bytes").unwrap();
    let cache = cache_dir(&root);
    let library = Fake::with(10);
    let found = search(&cache, &library, &query("cat"), 1).unwrap();
    let first = &found.candidates[0];
    let previews = cache.join("previews");
    std::fs::create_dir_all(&previews).unwrap();
    // The name the download used to write through: fixed, so anyone could
    // place a link there before the preview was ever asked for.
    let preview = format!("{}-{}", first.medium.word(), first.id);
    symlink(&victim, previews.join(format!("{preview}.part"))).unwrap();
    // And the next names it writes through now, made fresh: each is passed
    // over for one nobody placed.
    let landed = previews.join(format!("{preview}.jpg"));
    let seen = scratch_beside(&landed, "part").unwrap();
    let seen = seen.file_name().unwrap().to_string_lossy().into_owned();
    let (stem, number) = seen
        .strip_suffix(".part")
        .and_then(|s| s.rsplit_once('-'))
        .expect("<name>.<pid>-<n>.part");
    let number: u64 = number.parse().unwrap();
    for k in 1..=16 {
        symlink(
            &victim,
            previews.join(format!("{stem}-{}.part", number + k)),
        )
        .unwrap();
    }

    let shown = labelled_previews(&cache, &library, &found.candidates);

    assert_eq!(
        std::fs::read_to_string(&victim).unwrap(),
        "the victim's own bytes"
    );
    assert_eq!(shown[0].0, landed, "the preview still lands");
    assert_eq!(
        std::fs::read_to_string(&landed).unwrap(),
        format!("bytes of {}", first.preview_url)
    );
}
