//! `scorsese render`, end to end.
//!
//! `refusals` is the half that never spends anything: a shape we do not write
//! is turned down before ffmpeg is even located. The rest is the half that
//! does — real media in, a real file out, and every assertion made against
//! what ffprobe finds in that file rather than against what the command
//! printed. A setting that reached only the report would pass a test of the
//! text and produce the wrong film.

#[path = "../common/mod.rs"]
mod common;

mod failures;
mod notes;
mod outcome;
mod refusals;
mod settings;
mod sound;
mod synthesised;

use std::path::{Path, PathBuf};

use common::documents::{self, assets};
use common::media;
use common::{Run, run_in, temp_dir};

/// What one render wrote, and what the command said doing it.
pub(crate) struct Rendered {
    /// The file asked for. Present only if the render got that far.
    pub(crate) file: PathBuf,
    pub(crate) run: Run,
}

/// Renders a project to `name` inside its own directory, with real ffmpeg.
///
/// The output path is absolute because `--out` is resolved against the working
/// directory rather than the project — a test process shares its own with
/// every other test in the binary.
pub(crate) fn render(dir: &Path, name: &str, arguments: &[&str]) -> Rendered {
    let file = dir.join(name);
    let out = file.display().to_string();
    let mut all = vec!["render", "--out", &out];
    // The fixtures are tiny, and the default raster is a 1080p one. Filled in
    // here rather than by every caller, and only when nobody asked — a test of
    // `--resolution` has to be the one that decides it.
    if !arguments.contains(&"--resolution") {
        all.extend(["--resolution", media::SIZE]);
    }
    all.extend_from_slice(arguments);
    let run = run_in(dir, &all);
    Rendered { file, run }
}

impl Rendered {
    /// Asserts the render succeeded and hands back the file it wrote.
    #[track_caller]
    pub(crate) fn file(self) -> PathBuf {
        assert!(!self.run.failed, "the render failed:\n{}", self.run.output);
        assert!(
            self.file.is_file(),
            "the render reported success but wrote no {}",
            self.file.display()
        );
        self.file
    }
}

/// A project of one solid-colour shot over real media, `frames` long on a
/// 30 fps grid. The starting point of most of these: enough to encode, small
/// enough that doing so costs a fraction of a second.
pub(crate) fn one_shot(label: &str, frames: u32) -> PathBuf {
    shot_over(label, frames, |tools, dir| {
        media::colour_video(tools, dir, "red", "red", 2);
    })
}

/// The same, over a moving test pattern at a raster worth spending bits on —
/// the fixture for anything a flat colour would compress identically at every
/// setting.
pub(crate) fn pattern(label: &str, frames: u32) -> PathBuf {
    shot_over(label, frames, |tools, dir| {
        media::pattern_video(tools, dir, "red", 2);
    })
}

fn shot_over(label: &str, frames: u32, media: impl Fn(&scorsese_render::Tools, &Path)) -> PathBuf {
    let dir = temp_dir(label);
    media(&self::media::tools(), &dir);
    documents::write_into(
        &dir,
        &[assets::raw("red", "video", "mp4")],
        &[documents::video_track(&[documents::lasting(
            "c1", "red", 0, frames,
        )])],
    );
    dir
}
