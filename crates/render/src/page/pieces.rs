//! A long capture made in pieces at once, and joined into one file.
//!
//! The way Remotion renders a long composition (#809): each piece is a browser
//! of its own, run ahead to the piece's first frame without drawing
//! (`capture`'s), and then drawing its own stretch, while the others draw
//! theirs. Joined end to end their frames are the file one browser would have
//! drawn from the first frame to the last — every piece starts on a whole
//! millisecond ([`Request::step`]), so the join stamps each frame exactly where
//! a single capture would have.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::PageError;
use super::browser::Chrome;
use super::capture::{self, Heard, Served};
use super::request::Request;
use super::told::Read;
use crate::error::Stage;
use crate::tools::Tools;

/// The fewest seconds of page a piece of its own is worth: below it, starting
/// another browser and running it ahead costs more than the frames it draws.
const SHORTEST: f64 = 4.0;

/// How many browsers draw one capture at once, at most. Each is a process tree
/// of its own, a few hundred megabytes.
const MOST: usize = 4;

/// Captures each of `pieces` of `request`, each by a browser of its own and all
/// at once, into the one file `out`.
pub(crate) fn capture_in(
    chrome: &Chrome,
    tools: &Tools,
    served: Served<'_>,
    request: &Request,
    pieces: Vec<Range<u64>>,
    out: &Path,
) -> Result<Heard, PageError> {
    if let [only] = &pieces[..] {
        let only = only.clone();
        return capture::run(chrome, tools, served, request, (only, 0), out);
    }
    let files: Vec<PathBuf> = (0..pieces.len())
        .map(|i| with_suffix(out, &format!("{i}.mkv")))
        .collect();
    let heard: Vec<Result<Heard, PageError>> = std::thread::scope(|scope| {
        let running: Vec<_> = pieces
            .iter()
            .zip(&files)
            .enumerate()
            .map(|(i, (piece, file))| {
                let piece = piece.clone();
                // The first piece measures its layout from zero, the rest only
                // their own stretch: see `capture::run`.
                let measured = if i == 0 { 0 } else { piece.start };
                scope.spawn(move || {
                    capture::run(chrome, tools, served, request, (piece, measured), file)
                })
            })
            .collect();
        running
            .into_iter()
            .map(|thread| thread.join().expect("a capture thread does not panic"))
            .collect()
    });
    let joined = heard
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .and_then(|heard| {
            join(tools, request, &pieces, &files, out)?;
            Ok(merge(heard))
        });
    for file in &files {
        let _ = std::fs::remove_file(file);
    }
    joined
}

/// How many browsers the machine is worth: one a core, never more than
/// [`MOST`]. A capture comes before anything else in a render is drawn, so the
/// cores are its own; and the count is the one a container was granted, not
/// the host's. On four cores, a 90 s page at 640 × 360 rendered in 154 s from
/// one browser, 126 s from two and 105 s from four (#809).
pub(crate) fn ways() -> usize {
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    cores.clamp(1, MOST)
}

/// `frames` cut into at most `ways` pieces of at least [`SHORTEST`], each but
/// the last a whole number of [`Request::step`]s long, so every one starts on a
/// whole millisecond when `frames` does.
pub(crate) fn split(request: &Request, frames: Range<u64>, ways: usize) -> Vec<Range<u64>> {
    let step = request.step();
    let length = frames.end.saturating_sub(frames.start);
    let shortest = (SHORTEST * request.fps.as_f64()).ceil() as u64;
    let ways = (length / shortest.max(1)).clamp(1, ways.max(1) as u64);
    let each = length.div_ceil(ways).div_ceil(step) * step;
    let mut pieces = Vec::new();
    let mut start = frames.start;
    while start < frames.end {
        let end = (start + each).min(frames.end);
        pieces.push(start..end);
        start = end;
    }
    if pieces.is_empty() {
        pieces.push(frames);
    }
    pieces
}

/// Joins the pieces' files into `out` without decoding them: each is told how
/// long it lasts, to the millisecond it really does, so the one after it
/// starts where a single capture would have put it.
fn join(
    tools: &Tools,
    request: &Request,
    pieces: &[Range<u64>],
    files: &[PathBuf],
    out: &Path,
) -> Result<(), PageError> {
    // Named as they sit beside the list, which is where ffmpeg looks: a path
    // in a list is read from the list's folder, not from where ffmpeg runs.
    let mut list = String::new();
    for (piece, file) in pieces.iter().zip(files) {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        let quoted = name.replace('\'', r"'\''");
        list.push_str(&format!("file '{quoted}'\n"));
        let ms = (request.millis_at(piece.end) - request.millis_at(piece.start)).round();
        list.push_str(&format!("duration {ms}ms\n"));
    }
    let listed = with_suffix(out, "list");
    std::fs::write(&listed, list)?;
    let child = tools
        .ffmpeg()
        .args([
            "-nostdin", "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i",
        ])
        .arg(&listed)
        .args(["-c", "copy", "-f", "matroska"])
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| crate::RenderError::Spawn {
            stage: Stage::Encode,
            source,
        });
    let joined = child
        .and_then(|child| crate::pipe::finish(child, Stage::Encode, &out.display().to_string()));
    let _ = std::fs::remove_file(&listed);
    Ok(joined?)
}

/// What every piece heard, as one capture would have said it: every file any
/// of them loaded, every clip any of them read, and each warning once, in the
/// order first heard.
fn merge(heard: Vec<Heard>) -> Heard {
    let mut loaded = BTreeMap::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut read = Read::default();
    for piece in heard {
        loaded.extend(piece.loaded);
        read.merge(piece.read);
        for warning in piece.warnings {
            if !warnings.contains(&warning) {
                warnings.push(warning);
            }
        }
    }
    Heard {
        loaded,
        warnings,
        read,
    }
}

/// `out` with `suffix` added to its name, beside it.
fn with_suffix(out: &Path, suffix: &str) -> PathBuf {
    let mut name = out.file_name().unwrap_or_default().to_owned();
    name.push(format!(".{suffix}"));
    out.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use scorsese_compositor::Resolution;
    use scorsese_core::Fps;

    use super::*;

    fn request(fps: Fps, seconds: f64) -> Request {
        Request {
            page: "pages/a.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps,
            duration: seconds,
            clips: BTreeMap::new(),
        }
    }

    #[test]
    fn a_short_stretch_is_one_piece() {
        let short = request(Fps::THIRTY, 5.0);
        assert_eq!(split(&short, 0..151, 4), vec![0..151]);
        assert_eq!(split(&short, 60..65, 4), vec![60..65]);
    }

    #[test]
    fn a_long_stretch_is_cut_into_pieces_that_start_on_whole_milliseconds() {
        let ntsc = request(Fps::new(30000, 1001).unwrap(), 90.0);
        let frames = 0..ntsc.frames();
        let pieces = split(&ntsc, frames.clone(), 4);
        assert_eq!(pieces.len(), 4, "{pieces:?}");
        assert_eq!(pieces.first().unwrap().start, frames.start);
        assert_eq!(pieces.last().unwrap().end, frames.end);
        for pair in pieces.windows(2) {
            assert_eq!(pair[0].end, pair[1].start, "{pieces:?}");
        }
        for piece in &pieces {
            assert_eq!(piece.start % ntsc.step(), 0, "{pieces:?}");
        }
    }

    #[test]
    fn no_piece_is_shorter_than_it_is_worth_but_the_last() {
        let page = request(Fps::THIRTY, 10.0);
        let pieces = split(&page, 0..page.frames(), 8);
        assert_eq!(pieces.len(), 2, "{pieces:?}");
        assert_eq!(split(&page, 0..page.frames(), 1).len(), 1);
    }

    #[test]
    fn every_piece_is_heard_once() {
        let piece = |path: &str, warning: &str| Heard {
            loaded: BTreeMap::from([(path.to_owned(), None)]),
            warnings: vec![warning.to_owned(), "both".to_owned()],
            read: Read {
                names: [path.to_owned()].into(),
                listed: false,
            },
        };
        let heard = merge(vec![piece("a.png", "one"), piece("b.png", "two")]);
        assert_eq!(heard.loaded.len(), 2);
        assert_eq!(heard.read.names.len(), 2);
        assert_eq!(heard.warnings, ["one", "both", "two"]);
    }
}
