//! A slot's pieces: captures of only some of a page's frames (#809).
//!
//! A piece is `part-<first>-<end>.mkv` beside `part-<first>-<end>.json`, the
//! frames `first..end` of the page and their own [`Record`]. The whole page is
//! never a piece: asked for every frame, a capture is kept where [`super::cached`]
//! reads it, `frames.mkv` beside `capture.json`.

use std::ops::Range;
use std::path::{Path, PathBuf};

use super::{FRAMES, RECORD, Record, read_fresh};
use crate::page::request::Request;

/// The file a piece's frames are kept in starts with this.
const PART: &str = "part-";

/// Where a capture of `piece` is kept in `slot`: its frames and its record.
/// The whole page's frames are the slot's own capture; anything less, a piece.
pub(crate) fn files(slot: &Path, piece: &Range<u64>, request: &Request) -> (PathBuf, PathBuf) {
    if is_whole(piece, request) {
        return (slot.join(FRAMES), slot.join(RECORD));
    }
    let stem = format!("{PART}{}-{}", piece.start, piece.end);
    (
        slot.join(format!("{stem}.mkv")),
        slot.join(format!("{stem}.json")),
    )
}

/// The capture in `slot` that holds every frame `wanted` names and still
/// describes the project: the whole page's when it is fresh, and otherwise the
/// shortest fresh piece holding them. Returns its file, the page frame that
/// file begins at, and its record.
pub(crate) fn holding(
    slot: &Path,
    project_root: &Path,
    request: &Request,
    wanted: &Range<u64>,
) -> Option<(PathBuf, u64, Record)> {
    if let Some(record) = read_fresh(&slot.join(FRAMES), &slot.join(RECORD), project_root) {
        return Some((slot.join(FRAMES), 0, record));
    }
    let mut holders: Vec<Range<u64>> = pieces(slot)
        .into_iter()
        .filter(|piece| piece.start <= wanted.start && wanted.end <= piece.end)
        .collect();
    // Shortest first, so the freshness of a longer one is never hashed for
    // nothing.
    holders.sort_by_key(|piece| (piece.end - piece.start, piece.start));
    holders.into_iter().find_map(|piece| {
        let (file, record) = files(slot, &piece, request);
        read_fresh(&file, &record, project_root).map(|record| (file, piece.start, record))
    })
}

/// Drop every piece in `slot` that the capture of `kept`, just written, makes
/// unnecessary: all of them when `kept` is the whole page, and otherwise each
/// one `kept` holds all of, or that has gone stale. So an edit loop on one page
/// leaves one capture of each stretch looked at, not one per edit.
pub(crate) fn prune(slot: &Path, project_root: &Path, request: &Request, kept: &Range<u64>) {
    let whole = is_whole(kept, request);
    for piece in pieces(slot) {
        if piece == *kept {
            continue;
        }
        let (file, record) = files(slot, &piece, request);
        let covered = whole || (kept.start <= piece.start && piece.end <= kept.end);
        if covered || read_fresh(&file, &record, project_root).is_none() {
            // Record first: frames without one are never read, so a removal
            // cut short leaves nothing that looks usable.
            let _ = std::fs::remove_file(record);
            let _ = std::fs::remove_file(file);
        }
    }
}

/// Whether `piece` is every frame of the page.
fn is_whole(piece: &Range<u64>, request: &Request) -> bool {
    piece.start == 0 && piece.end >= request.frames()
}

/// Every piece whose frames are in `slot`, by the frames it holds.
fn pieces(slot: &Path) -> Vec<Range<u64>> {
    let Ok(read) = std::fs::read_dir(slot) else {
        return Vec::new();
    };
    read.flatten()
        .filter_map(|entry| parse(entry.file_name().to_str()?))
        .collect()
}

/// `part-<first>-<end>.mkv` as `first..end`; anything else — a record, a
/// partial capture, the whole page's frames — as nothing.
fn parse(name: &str) -> Option<Range<u64>> {
    let (first, end) = name
        .strip_prefix(PART)?
        .strip_suffix(".mkv")?
        .split_once('-')?;
    let (first, end): (u64, u64) = (first.parse().ok()?, end.parse().ok()?);
    (first < end).then_some(first..end)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use scorsese_compositor::Resolution;
    use scorsese_core::{Fps, hash_bytes};

    use super::super::keep;
    use super::*;

    /// A one-second page at 30 fps: 31 frames.
    fn request() -> Request {
        Request {
            page: "page.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps: Fps::THIRTY,
            duration: 1.0,
        }
    }

    /// A project whose page reads `text`, with an empty slot.
    fn project(name: &str, text: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("scorsese-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let slot = root.join("slot");
        std::fs::create_dir_all(&slot).unwrap();
        std::fs::write(root.join("page.html"), text).unwrap();
        (root, slot)
    }

    /// Writes a capture of `piece` that loaded the page as reading `text`.
    fn capture(slot: &Path, piece: Range<u64>, text: &str) -> PathBuf {
        let (file, record) = files(slot, &piece, &request());
        std::fs::write(&file, format!("{piece:?}")).unwrap();
        let loaded = BTreeMap::from([("page.html".into(), Some(hash_bytes(text.as_bytes())))]);
        keep(
            &record,
            &Record {
                loaded,
                warnings: vec![format!("{piece:?}")],
            },
        )
        .unwrap();
        file
    }

    #[test]
    fn the_whole_page_is_the_slot_s_own_capture_and_anything_less_a_piece() {
        let slot = Path::new("/s");
        assert_eq!(
            files(slot, &(0..31), &request()),
            (slot.join(FRAMES), slot.join(RECORD))
        );
        assert_eq!(
            files(slot, &(3..31), &request()),
            (slot.join("part-3-31.mkv"), slot.join("part-3-31.json"))
        );
        assert_eq!(
            files(slot, &(0..30), &request()),
            (slot.join("part-0-30.mkv"), slot.join("part-0-30.json"))
        );
    }

    #[test]
    fn only_a_piece_s_frames_are_read_as_one() {
        assert_eq!(parse("part-3-31.mkv"), Some(3..31));
        for other in [
            "part-3-31.json",
            "part-3-31.123.partial.mkv",
            "part-31-3.mkv",
            "part-3.mkv",
            "frames.mkv",
            "fonts.conf",
        ] {
            assert_eq!(parse(other), None, "{other}");
        }
    }

    #[test]
    fn the_whole_capture_comes_first_then_the_shortest_fresh_piece() {
        let (root, slot) = project("holding", "one");
        let request = request();
        assert_eq!(holding(&slot, &root, &request, &(10..12)), None);
        capture(&slot, 0..21, "one");
        let short = capture(&slot, 9..15, "one");
        capture(&slot, 12..30, "one");
        let (file, first, record) = holding(&slot, &root, &request, &(10..12)).unwrap();
        assert_eq!((file, first), (short, 9));
        assert_eq!(record.warnings, ["9..15"]);
        assert_eq!(
            holding(&slot, &root, &request, &(10..16)).map(|(_, first, _)| first),
            Some(0),
            "the only piece holding all of them"
        );
        assert_eq!(holding(&slot, &root, &request, &(20..31)), None);
        // A stale piece is passed over for a longer fresh one.
        capture(&slot, 10..12, "edited");
        assert_eq!(
            holding(&slot, &root, &request, &(10..12)).map(|(_, first, _)| first),
            Some(9)
        );
        let whole = capture(&slot, 0..31, "one");
        assert_eq!(
            holding(&slot, &root, &request, &(10..12)).map(|(file, first, _)| (file, first)),
            Some((whole, 0))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_piece_goes_once_stale_or_held_by_a_newer_capture() {
        let (root, slot) = project("prune", "two");
        let request = request();
        let stale = capture(&slot, 0..6, "one");
        let inside = capture(&slot, 9..12, "two");
        let overlapping = capture(&slot, 3..15, "two");
        let kept = capture(&slot, 6..15, "two");
        prune(&slot, &root, &request, &(6..15));
        assert!(!stale.exists() && !inside.exists());
        assert!(!stale.with_extension("json").exists() && !inside.with_extension("json").exists());
        assert!(overlapping.exists(), "it holds frames the new one does not");
        assert!(kept.exists() && kept.with_extension("json").exists());
        capture(&slot, 0..31, "two");
        prune(&slot, &root, &request, &(0..31));
        assert_eq!(pieces(&slot), Vec::<Range<u64>>::new());
        assert!(slot.join(FRAMES).exists() && slot.join(RECORD).exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
