//! A link waiting where a page capture writes before its frames land is never
//! written through (#1672): the page cache is a folder a member's content can
//! place entries in, and a `.scor` received from someone else can carry one.
//!
//! Needs the pinned browser, like `pages.rs` beside it.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use scorsese_core::Fps;
use scorsese_render::Resolution;
use scorsese_render::page::{self, Request};

use crate::common::ffmpeg::tools;
use crate::common::planting;
use crate::common::scratch;

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

/// Links at each of the next names a capture of `frames` into `slot` writes
/// before its frames land: to `victim` where it wrote on main (the partial
/// file, and the pieces and the list a capture in pieces writes beside it),
/// and to `outside` where it writes now (its scratch folder).
fn plant(slot: &Path, frames: Range<u64>, victim: &Path, outside: &Path) -> Vec<PathBuf> {
    let stem = slot.join(format!("part-{}-{}", frames.start, frames.end));
    let mut planted = planting::plant(&stem, "partial.mkv", &[".list", ".0.mkv", ".1.mkv"], victim);
    planted.extend(planting::plant(&stem, "capturing", &[], outside));
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
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(
        dir.join("pages/flat.html"),
        "<style>div { position: absolute; inset: 0; background: #00f }</style><div></div>",
    )
    .expect("a page");
    let victim = outside.join("victim.txt");
    std::fs::write(&victim, "the victim's own bytes").expect("a victim");
    let frames = 0..request.frames();

    let first = page::capture(&chrome, &tools, &dir, request).expect("captured");
    let shelf = first.file.parent().expect("a shelf").to_path_buf();
    let slot = shelf.parent().expect("a slot").to_path_buf();
    std::fs::remove_dir_all(&shelf).expect("the shelf goes");
    let planted = plant(&slot, frames.clone(), &victim, &outside);

    let again = page::capture_frames(
        &chrome,
        &tools,
        &dir,
        page::Laid::default(),
        request,
        frames,
        browsers,
    )
    .expect("captured past the links");

    assert!(
        std::fs::read(&victim).expect("the victim") == b"the victim's own bytes",
        "the link's target was written through"
    );
    assert_eq!(
        std::fs::read_dir(&outside).expect("outside").count(),
        1,
        "nothing was written through a linked folder"
    );
    let landed = std::fs::symlink_metadata(&again.file).expect("the frames");
    assert!(
        landed.is_file() && landed.len() > 0,
        "the frames still land"
    );
    let left: Vec<String> = std::fs::read_dir(&slot)
        .expect("the slot")
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
