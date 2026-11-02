//! A link waiting where a page capture writes before its frames land is never
//! written through (#1672): the page cache is a folder a member's content can
//! place entries in, and a `.scor` received from someone else can carry one.
//!
//! Needs the pinned browser, like `pages.rs` beside it.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use scorsese_core::Fps;
use scorsese_render::Resolution;
use scorsese_render::page::{self, Request};

use crate::common::ffmpeg::tools;
use crate::common::scratch;

/// How many of the next scratch names get a link: more than one capture
/// takes, so a name another call in this process took does not slip past.
const PLANTED: u64 = 16;

fn request(fps: Fps, duration: f64) -> Request {
    Request {
        page: "pages/flat.html".into(),
        resolution: Resolution::new(64, 64).expect("a raster"),
        fps,
        duration,
        clips: BTreeMap::new(),
        words: BTreeMap::new(),
    }
}

/// A link to `victim` at each of the next names a capture of `frames` into
/// `slot` would write before its frames land — the partial file, and the
/// pieces and the list a capture in pieces writes beside it.
fn plant(slot: &Path, frames: std::ops::Range<u64>, victim: &Path) -> Vec<PathBuf> {
    let stem = slot.join(format!("part-{}-{}", frames.start, frames.end));
    let seen = scorsese_core::write::scratch_beside(&stem, "partial.mkv").unwrap();
    let seen = seen.file_name().unwrap().to_string_lossy().into_owned();
    let (name, number) = seen
        .strip_suffix(".partial.mkv")
        .and_then(|s| s.rsplit_once('-'))
        .expect("<name>.<pid>-<n>.partial.mkv");
    let number: u64 = number.parse().unwrap();
    let mut planted = Vec::new();
    for k in 1..=PLANTED {
        let partial = format!("{name}-{}.partial.mkv", number + k);
        for sibling in ["", ".list", ".0.mkv", ".1.mkv"] {
            let link = slot.join(format!("{partial}{sibling}"));
            symlink(victim, &link).unwrap();
            planted.push(link);
        }
    }
    planted
}

/// Captures `request` once to find its slot, empties the slot, plants links
/// there and captures again with `browsers`: the victim keeps its bytes and
/// the frames still land.
fn captured_past_links(label: &str, request: &Request, browsers: usize) {
    let chrome = scorsese_render::testing::chrome();
    let tools = tools();
    let dir = scratch::dir(label);
    let outside = scratch::dir(&format!("{label}-outside"));
    std::fs::create_dir_all(dir.join("pages")).unwrap();
    std::fs::write(
        dir.join("pages/flat.html"),
        "<style>div { position: absolute; inset: 0; background: #00f }</style><div></div>",
    )
    .unwrap();
    let victim = outside.join("victim.txt");
    std::fs::write(&victim, "the victim's own bytes").unwrap();
    let frames = 0..request.frames();

    let first = page::capture(&chrome, &tools, &dir, request).expect("captured");
    let shelf = first.file.parent().unwrap().to_path_buf();
    let slot = shelf.parent().unwrap().to_path_buf();
    std::fs::remove_dir_all(&shelf).unwrap();
    let planted = plant(&slot, frames.clone(), &victim);

    let again = page::capture_frames(&chrome, &tools, &dir, &[], request, frames, browsers)
        .expect("captured past the links");

    assert!(
        std::fs::read(&victim).unwrap() == b"the victim's own bytes",
        "the link's target was written through"
    );
    let landed = std::fs::symlink_metadata(&again.file).unwrap();
    assert!(
        landed.is_file() && landed.len() > 0,
        "the frames still land"
    );
    let left: Vec<String> = std::fs::read_dir(&slot)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| !planted.contains(path) && !path.starts_with(&shelf))
        .map(|path| path.display().to_string())
        .collect();
    assert!(left.is_empty(), "the capture left scratch behind: {left:?}");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&outside).ok();
}

#[test]
fn a_link_where_a_capture_writes_is_left_as_it_was() {
    captured_past_links("planted-capture", &request(Fps::THIRTY, 0.5), 1);
}

#[test]
fn a_link_where_a_capture_in_pieces_writes_is_left_as_it_was() {
    let fps = Fps::new(4, 1).expect("a rate");
    captured_past_links("planted-pieces", &request(fps, 9.0), 2);
}
