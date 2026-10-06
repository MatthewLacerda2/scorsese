//! `scorsese still`
//!
//! One frame, composited and written out, with no encoder anywhere near it.
//!
//! The distinction from `render --stills` is the whole point of the command and
//! is worth stating where someone will read it. `--stills` renders the file and
//! then decodes frames back out of it, which is the right answer to *what did
//! the encoder actually produce* — and the wrong one to *what does this frame
//! look like*, since it charges a whole render and a whole encode for a
//! twenty-fourth of a second. This is that second question: the same plan, the
//! same decoders, the same compositor, stopped one step before the encode.
//!
//! `--sheet` writes the instants as one contact sheet rather than a PNG each
//! (#814): the frames side by side, labelled, in the one file `--out` names —
//! what a preview taken in the background otherwise had to be tiled into by
//! hand before anybody could compare them.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use scorsese_core::{Frames, Project};
use scorsese_render::{Cue, RenderSettings, Renderer, Resolution, Tools, contact, frames};

/// The raster a frame is composited at when nobody says: the delivery size most
/// cuts are made for, since a PNG written here is often kept.
const DEFAULT_RASTER: (u32, u32) = (1920, 1080);

/// What a sheet's cell is composited at when nobody says — five of them side by
/// side are about as wide as one full frame, and a title in one still reads.
const DEFAULT_CELL: (u32, u32) = (640, 360);

/// How the frames are drawn and written, beyond which instants they are.
pub(crate) struct Options {
    /// The raster to composite at — each cell's, with `sheet`. `None` takes
    /// the default for the mode.
    pub(crate) resolution: Option<Resolution>,
    /// Rule each frame (or cell) with coordinates.
    pub(crate) grid: bool,
    /// Tile the instants into one contact sheet, written to exactly `out`.
    pub(crate) sheet: bool,
}

/// Composites each of `at` and writes it out as a PNG under `out`.
///
/// Instants are sorted and de-duplicated, so naming one twice composites it
/// once — the same bargain `render --stills` makes, for the same reason: an
/// agent that asks about a boundary and about the time that boundary falls at
/// has asked one question.
///
/// `grid` rules each frame with coordinates before it is written. The PNG is
/// the only thing this command produces, so that is where the ruler goes: a
/// flag kept off the one file it makes would do nothing at all.
///
/// `sheet` tiles them into one PNG at `out` instead, in the same sorted order.
pub(crate) fn run(project_dir: &Path, out: &Path, at: &[Cue], options: Options) -> Result<()> {
    let Options {
        resolution,
        grid,
        sheet,
    } = options;
    let resolution = match resolution {
        Some(asked) => asked,
        None => {
            let (width, height) = if sheet { DEFAULT_CELL } else { DEFAULT_RASTER };
            Resolution::new(width, height).context("the default raster")?
        }
    };
    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    // The project's own grid, always. Nothing is being encoded, so there is no
    // output rate for a time to conform to, and reading `2.5s` against any
    // other rate would name an instant the edit does not have.
    let fps = project.timeline_fps;

    let mut wanted: Vec<Frames> = at.iter().map(|cue| cue.timeline_frame(fps)).collect();
    wanted.sort_unstable();
    wanted.dedup();

    let tools = Tools::discover()?;
    // Everything a still cannot be affected by is left at its default and never
    // offered: a bitrate, a codec and a container all describe a file that is
    // never written here. A flag that could not change the picture should not
    // be a flag.
    let renderer = Renderer::new(&tools, RenderSettings::new(resolution, fps));
    if sheet {
        return write_sheet(&tools, &renderer, &project, project_dir, out, &wanted, grid);
    }
    let several = wanted.len() > 1;

    let mut told = Vec::new();
    for frame in wanted {
        let (mut picture, notes) = renderer
            .still_noted(&project, project_dir, frame)
            .with_context(|| format!("compositing timeline frame {}", frame.get()))?;
        // After compositing, over the finished frame — the ruler is furniture
        // for reading the picture, not a layer of it.
        if grid {
            scorsese_render::grid::draw(&mut picture);
        }
        let path = destination(out, frame, several);
        frames::write_png(&tools, &path, &picture)
            .with_context(|| format!("writing {}", path.display()))?;
        println!(
            "Wrote {} — frame {} ({:.2}s) at {resolution}{}",
            path.display(),
            frame.get(),
            fps.seconds(frame),
            if grid { ", ruled in fractions" } else { "" }
        );
        // As `render` prints them, and once: a page on screen at several
        // instants is one page with one problem.
        for note in notes {
            if !told.contains(&note) {
                println!("  note: {note}");
                told.push(note);
            }
        }
    }
    Ok(())
}

/// Composites `wanted` into one contact sheet and writes it to exactly `out`.
fn write_sheet(
    tools: &Tools,
    renderer: &Renderer<'_>,
    project: &Project,
    project_dir: &Path,
    out: &Path,
    wanted: &[Frames],
    grid: bool,
) -> Result<()> {
    let fps = project.timeline_fps;
    let drawn = contact::timeline_sheet(renderer, project, project_dir, wanted, grid)?;
    frames::write_png(tools, out, &drawn.image)
        .with_context(|| format!("writing {}", out.display()))?;
    let cells: Vec<String> = wanted
        .iter()
        .map(|frame| format!("{} ({:.2}s)", frame.get(), fps.seconds(*frame)))
        .collect();
    println!(
        "Wrote {} — a sheet of frames {}{}",
        out.display(),
        cells.join(", "),
        if grid {
            ", each ruled in fractions"
        } else {
            ""
        }
    );
    // Keyed by the instant, since a sheet has no "under the frame" to put a
    // note — and once, as the separate PNGs print them.
    let mut told = Vec::new();
    for (frame, notes) in drawn.notes {
        for note in notes {
            if !told.contains(&note) {
                println!("  note at frame {}: {note}", frame.get());
                told.push(note);
            }
        }
    }
    Ok(())
}

/// Where one frame's PNG goes.
///
/// One instant writes exactly the file that was asked for: someone who says
/// `--out frame.png` and means one frame should get `frame.png`, not a name
/// this command invented. Several cannot all be that file, so each takes its
/// timeline frame — zero-padded, so a directory listing is in playback order,
/// which is the same reasoning `render --stills` names its files by.
fn destination(out: &Path, frame: Frames, several: bool) -> PathBuf {
    if !several {
        return out.to_path_buf();
    }
    let mut name = out.file_stem().unwrap_or_default().to_os_string();
    name.push(format!("-{:05}", frame.get()));
    if let Some(extension) = out.extension() {
        name.push(".");
        name.push(extension);
    }
    out.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_instant_writes_exactly_the_file_that_was_asked_for() {
        let out = Path::new("review/frame.png");
        assert_eq!(
            destination(out, Frames(285), false),
            PathBuf::from("review/frame.png")
        );
    }

    #[test]
    fn several_instants_take_the_frame_they_are_of() {
        let out = Path::new("review/frame.png");
        assert_eq!(
            destination(out, Frames(285), true),
            PathBuf::from("review/frame-00285.png")
        );
        assert_eq!(
            destination(out, Frames::ZERO, true),
            PathBuf::from("review/frame-00000.png"),
            "zero-padded, so a listing is in playback order"
        );
    }

    /// A name with no extension, and one with two dots in it. Neither is worth
    /// refusing, and both have an obvious right answer.
    #[test]
    fn a_name_without_an_extension_still_gets_its_frame_number() {
        assert_eq!(
            destination(Path::new("frame"), Frames(7), true),
            PathBuf::from("frame-00007")
        );
        assert_eq!(
            destination(Path::new("my.cut.png"), Frames(7), true),
            PathBuf::from("my.cut-00007.png")
        );
    }
}
