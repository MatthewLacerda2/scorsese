//! `scorsese still`, end to end.
//!
//! The command exists to be *looked at*, so the assertions are about pixels
//! wherever they can be: a still of a red shot is checked by reading the PNG
//! back and finding red in it, not by trusting the line the command printed. A
//! path that composited black and reported success would pass every test of the
//! text and be worthless in the one job this has.
//!
//! `files` is the other half — where the PNGs land, and what happens when an
//! instant is not one the edit has.

#[path = "../common/mod.rs"]
mod common;

mod files;
mod pixels;

use std::path::{Path, PathBuf};

use common::documents::{self, assets};
use common::{Run, media, run_in, temp_dir};

/// The raster the stills are taken at: small, because what is being checked is
/// which pixels come out rather than how many.
pub(crate) const RASTER: &str = "64x36";

/// A project of one title over nothing — the cheapest thing that composites,
/// since a `text` asset needs no file on disk and no decoder.
pub(crate) fn titled(label: &str) -> PathBuf {
    documents::project(
        label,
        &[assets::text("title", "TEASER")],
        &[documents::lasting("c1", "title", 0, 300)],
    )
}

/// The same shape over real media, for the tests that have to see a colour.
pub(crate) fn shot(label: &str, colour: &str) -> PathBuf {
    let dir = temp_dir(label);
    media::colour_video(&media::tools(), &dir, "shot", colour, 2);
    documents::write_into(
        &dir,
        &[assets::raw("shot", "video", "mp4")],
        &[documents::video_track(&[documents::lasting(
            "c1", "shot", 0, 60,
        )])],
    );
    dir
}

/// Runs `still` against `dir`, writing under it so the test cleans up with it.
pub(crate) fn still(dir: &Path, out: &str, arguments: &[&str]) -> Run {
    let path = dir.join(out).display().to_string();
    let mut all = vec!["still", "--out", &path];
    if !arguments.contains(&"--resolution") {
        all.extend(["--resolution", RASTER]);
    }
    all.extend_from_slice(arguments);
    run_in(dir, &all)
}
