//! A long capture made a part at a time, in turns of its own (#1708).
//!
//! The web's capture launcher has one slot every account shares, so no turn
//! may draw more than so many frames: a longer stretch is cut into parts,
//! each captured as a piece of the slot — run ahead to its first frame, as
//! any piece is (`capture`'s) — with other pages captured between them. Once
//! the last part is in, they are joined into the one capture a single turn
//! would have drawn, the way pieces drawn at once are (`pieces`'), so a
//! render reads them as one.

use std::ops::Range;

use super::request::Request;

/// `frames` of `request` cut for turns of at most `most` frames drawn: the
/// stretch a capture of them draws ([`super::capture_frames`]'s), in
/// consecutive parts that each start on a whole millisecond
/// ([`Request::step`]), so they join without moving a frame. A part is a
/// whole number of steps long, so one step when `most` is shorter.
pub fn parts(request: &Request, frames: Range<u64>, most: u64) -> Vec<Range<u64>> {
    let span = super::piece(request, frames);
    let step = request.step();
    let each = (most / step).max(1) * step;
    let mut parts = Vec::new();
    let mut start = span.start;
    while start < span.end {
        let end = (start + each).min(span.end);
        parts.push(start..end);
        start = end;
    }
    if parts.is_empty() {
        parts.push(span);
    }
    parts
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

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
            words: BTreeMap::new(),
        }
    }

    #[test]
    fn a_stretch_no_longer_than_a_turn_is_one_part() {
        let page = request(Fps::new(10, 1).unwrap(), 179.9);
        assert_eq!(page.frames(), 1800);
        assert_eq!(parts(&page, 0..1800, 1800), [0..1800]);
        assert_eq!(parts(&page, 40..45, 1800), [40..45]);
    }

    #[test]
    fn a_longer_stretch_is_cut_into_turns_from_where_its_capture_starts() {
        let page = request(Fps::new(10, 1).unwrap(), 199.9);
        assert_eq!(page.frames(), 2000);
        assert_eq!(parts(&page, 0..2000, 1800), [0..1800, 1800..2000]);
        let ntsc = request(Fps::new(30000, 1001).unwrap(), 200.0);
        let step = ntsc.step();
        let cut = parts(&ntsc, 31..ntsc.frames(), 1800);
        assert_eq!(cut.first().unwrap().start, ntsc.piece_start(31));
        assert_eq!(cut.last().unwrap().end, ntsc.frames());
        for pair in cut.windows(2) {
            assert_eq!(pair[0].end, pair[1].start, "{cut:?}");
        }
        for part in &cut {
            assert_eq!(part.start % step, 0, "{cut:?}");
            assert!(part.end - part.start <= 1800, "{cut:?}");
        }
    }

    #[test]
    fn a_turn_shorter_than_a_step_is_one_step() {
        let ntsc = request(Fps::new(30000, 1001).unwrap(), 10.0);
        let step = ntsc.step();
        assert!(step > 1);
        assert_eq!(parts(&ntsc, 0..step * 2, 1), [0..step, step..step * 2]);
    }
}
