//! A slot's pieces: captures of only some of a page's frames (#809).
//!
//! A piece is `part-<first>-<end>.mkv` beside `part-<first>-<end>.json`, the
//! frames `first..end` of the page and their own [`Record`]. The whole page is
//! never a piece: asked for every frame, a capture is kept where [`super::cached`]
//! reads it, `frames.mkv` beside `capture.json`.
//!
//! Everything here works in one of a slot's shelves (`super`'s): the `slot`
//! each function takes is that shelf's folder.

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
pub(super) fn holding(
    slot: &Path,
    project_root: &Path,
    request: &Request,
    wanted: &Range<u64>,
) -> Option<(PathBuf, u64, Record)> {
    if let Some(record) = read_fresh(
        &slot.join(FRAMES),
        &slot.join(RECORD),
        project_root,
        request,
    ) {
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
        read_fresh(&file, &record, project_root, request).map(|record| (file, piece.start, record))
    })
}

/// Every piece in `slot` no longer worth keeping, by its frames' and its
/// record's files: all of them when the whole page's capture is fresh, and
/// otherwise each one gone stale or held all of by another fresh piece. So an
/// edit loop on one page leaves one capture of each stretch looked at, not one
/// per edit. Deleting them is the caller's, which knows what is still held.
pub(crate) fn unneeded(
    slot: &Path,
    project_root: &Path,
    request: &Request,
) -> Vec<(PathBuf, PathBuf)> {
    let fresh = |piece: &Range<u64>| {
        let (file, record) = files(slot, piece, request);
        read_fresh(&file, &record, project_root, request).is_some()
    };
    let whole = read_fresh(
        &slot.join(FRAMES),
        &slot.join(RECORD),
        project_root,
        request,
    )
    .is_some();
    let all = pieces(slot);
    let kept: Vec<&Range<u64>> = all.iter().filter(|piece| fresh(piece)).collect();
    all.iter()
        .filter(|piece| {
            whole
                || !kept.contains(piece)
                || kept.iter().any(|other| {
                    other != piece && other.start <= piece.start && piece.end <= other.end
                })
        })
        .map(|piece| files(slot, piece, request))
        .collect()
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
    use crate::page::told::Told;

    /// A one-second page at 30 fps: 31 frames.
    fn request() -> Request {
        Request {
            page: "page.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps: Fps::THIRTY,
            duration: 1.0,
            clips: BTreeMap::new(),
            words: BTreeMap::new(),
        }
    }

    /// A project whose page reads `text`, with an empty slot.
    fn project(name: &str, text: &str) -> (PathBuf, PathBuf) {
        let root = crate::scratch::fresh(&format!("scorsese-{name}"), "");
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
                told: Told::default(),
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
            "part-3-31.123-4.partial.mkv",
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
    fn a_piece_goes_once_stale_or_held_by_another_fresh_capture() {
        let (root, slot) = project("unneeded", "two");
        let request = request();
        let unneeded = || -> Vec<PathBuf> {
            let mut files: Vec<PathBuf> = unneeded(&slot, &root, &request)
                .into_iter()
                .map(|(file, record)| {
                    assert_eq!(record, file.with_extension("json"));
                    file
                })
                .collect();
            files.sort();
            files
        };
        assert_eq!(unneeded(), Vec::<PathBuf>::new());
        let stale = capture(&slot, 0..6, "one");
        let inside = capture(&slot, 9..12, "two");
        let overlapping = capture(&slot, 3..15, "two");
        let other = capture(&slot, 6..20, "two");
        let mut expected = vec![stale, inside];
        expected.sort();
        assert_eq!(
            unneeded(),
            expected,
            "an overlapping piece holds frames the others do not"
        );
        capture(&slot, 0..31, "two");
        let mut every = vec![
            slot.join("part-0-6.mkv"),
            slot.join("part-9-12.mkv"),
            overlapping,
            other,
        ];
        every.sort();
        assert_eq!(unneeded(), every, "the whole page holds every piece");
        let _ = std::fs::remove_dir_all(&root);
    }
}
