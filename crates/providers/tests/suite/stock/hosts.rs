//! A download goes only to the library's own hosts, over https, whatever page
//! named it: a page in the cache is a file anything on the disk could have
//! written, so what it says is never taken as the vendor's word for where a
//! file is.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use scorsese_providers::credentials::Secret;
use scorsese_providers::stock::{
    Candidate, Choice, Library, LottieLibrary, Medium, PixabayLibrary, cache_dir, footage, import,
    keep, previews,
};
use serde_json::json;

use crate::common::project;
use crate::stock::fake::{Fake, candidate, probe};

/// Writes `candidates` into `cache` as a fresh page, the way something other
/// than a search would have to.
fn plant(cache: &Path, candidates: &[Candidate]) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let page = json!({
        "listing": "planted",
        "fetched_unix": now,
        "candidates": candidates,
        "total": candidates.len(),
    });
    std::fs::create_dir_all(cache.join("pages")).expect("the pages folder");
    std::fs::write(cache.join("pages/planted.json"), page.to_string()).expect("the page");
}

/// `medium` `id`, every file it names at `url`.
fn at(medium: Medium, id: u64, url: &str) -> Candidate {
    let mut one = candidate(medium, id, false);
    for rendition in &mut one.renditions {
        rendition.url = url.to_owned();
    }
    one.preview_url = url.to_owned();
    if one.motion_url.is_some() {
        one.motion_url = Some(url.to_owned());
    }
    one
}

/// Imports `ids` of `medium` from `library`, and says each refusal.
fn imported(root: &Path, library: &dyn Library, medium: Medium, ids: &[u64]) -> Vec<String> {
    let mut project = scorsese_core::Project::load(root).expect("the project");
    let choices: Vec<Choice> = ids.iter().map(|&id| Choice { medium, id }).collect();
    let answers = import(
        &mut project,
        root,
        &cache_dir(root),
        library,
        &choices,
        (1920, 1080),
        &probe(),
    );
    answers
        .into_iter()
        .map(|answer| answer.expect_err("refused").to_string())
        .collect()
}

#[test]
fn a_file_off_the_librarys_hosts_or_off_https_is_never_asked_for() {
    let (root, _) = project("stock-hosts-fake");
    let cache = cache_dir(&root);
    let library = Fake::with(0);
    let planted = [
        at(Medium::Video, 4, "https://elsewhere.example.invalid/4.mp4"),
        at(Medium::Video, 5, "http://cdn.example.invalid/5.mp4"),
        at(
            Medium::Lottie,
            6,
            "https://elsewhere.example.invalid/6.json",
        ),
    ];
    plant(&cache, &planted);

    for said in imported(&root, &library, Medium::Video, &[4, 5]) {
        assert!(said.contains("nothing was downloaded"), "{said}");
    }
    let kept = keep(&root, &cache, &library, &[6]);
    let said = kept[0].as_ref().expect_err("refused").to_string();
    assert!(said.contains("nothing was downloaded"), "{said}");
    let looked = footage(&cache, &library, Medium::Lottie, 6).expect_err("refused");
    assert!(
        looked.to_string().contains("nothing was downloaded"),
        "{looked}"
    );
    assert!(
        previews(&cache, &library, &planted)
            .iter()
            .all(Option::is_none)
    );
    assert_eq!(library.downloads.get(), 0, "nothing was asked of anywhere");
}

#[test]
fn pixabay_and_lottiefiles_ask_nothing_of_a_host_that_is_not_theirs() {
    let (root, _) = project("stock-hosts-vendors");
    let cache = cache_dir(&root);
    // Every one of these would fail to connect if it were asked: none names a
    // vendor's real host, so a regression is a refusal of the wrong kind,
    // never a call to a vendor.
    let planted = [
        at(Medium::Video, 7, "https://127.0.0.1:9/7.mp4"),
        at(Medium::Video, 8, "https://cdn.pixabay.com.invalid/8.mp4"),
        at(
            Medium::Video,
            9,
            "https://cdn.pixabay.com@127.0.0.1:9/9.mp4",
        ),
        at(Medium::Video, 10, "https://notpixabay.com.invalid/10.mp4"),
        at(
            Medium::Lottie,
            11,
            "https://lottiefiles.com.invalid/11.json",
        ),
    ];
    plant(&cache, &planted);

    let pixabay = PixabayLibrary::new(&Secret::new("unused"));
    for said in imported(&root, &pixabay, Medium::Video, &[7, 8, 9, 10]) {
        assert!(said.contains("nothing was downloaded"), "{said}");
    }
    let kept = keep(&root, &cache, &LottieLibrary::new(), &[11]);
    let said = kept[0].as_ref().expect_err("refused").to_string();
    assert!(said.contains("nothing was downloaded"), "{said}");
}
