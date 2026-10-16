//! A render's page clips captured together, under one browser budget (#1095).
//!
//! Each distinct page request is captured on a thread of its own, its
//! browsers leased from a [`page::Browsers`] every one of them shares — the
//! same ceiling one long capture is split under ([`page::browsers`]), counted
//! across clips. A dozen short pages, each worth a browser or two, then draw
//! side by side instead of one after another.
//!
//! Clips asking for the same request stay on one thread, in order, so a later
//! one still finds what an earlier one captured instead of drawing it again.
//! What each clip is handed comes back in the order the clips were asked for,
//! so the report reads the same however the threads finished.

use std::ops::Range;
use std::path::Path;
use std::sync::atomic::AtomicU64;

use crate::cancel::Cancel;
use crate::page::{self, Captured, Chrome, Request};
use crate::tools::Tools;

/// What one clip's capture came to: the file, or why there is none.
pub(super) type Outcome = Result<Captured, String>;

/// Where the captures are made, and how: the browser and tools, the project
/// and the counter each frame drawn is added to, and the cancel that stops
/// them all.
pub(super) struct Capturing<'a> {
    pub(super) chrome: &'a Chrome,
    pub(super) tools: &'a Tools,
    pub(super) project_root: &'a Path,
    pub(super) drawn: &'a AtomicU64,
    pub(super) cancel: &'a Cancel,
}

/// Captures every one of `asked`, distinct requests at once under one budget
/// of `browsers`: each clip's outcome in `asked`'s order, and the holdings
/// that keep the files from the cache's tidy until the render is over.
pub(super) fn capture(
    how: &Capturing<'_>,
    asked: &[(Request, Range<u64>)],
    browsers: &page::Browsers,
) -> (Vec<Outcome>, Vec<page::Holding>) {
    let groups = by_request(asked);
    let done: Vec<(page::Holding, Vec<(usize, Outcome)>)> = std::thread::scope(|scope| {
        let running: Vec<_> = groups
            .iter()
            .map(|group| scope.spawn(move || in_turn(how, asked, group, browsers)))
            .collect();
        running
            .into_iter()
            .map(|thread| thread.join().expect("a capture thread does not panic"))
            .collect()
    });
    let mut outcomes: Vec<Option<Outcome>> = asked.iter().map(|_| None).collect();
    let mut holdings = Vec::new();
    for (holding, captured) in done {
        holdings.push(holding);
        for (i, outcome) in captured {
            outcomes[i] = Some(outcome);
        }
    }
    let outcomes = outcomes
        .into_iter()
        .map(|outcome| outcome.expect("every clip asked is in one group"))
        .collect();
    (outcomes, holdings)
}

/// The clips of one request, captured one after another into one holding.
fn in_turn(
    how: &Capturing<'_>,
    asked: &[(Request, Range<u64>)],
    group: &[usize],
    browsers: &page::Browsers,
) -> (page::Holding, Vec<(usize, Outcome)>) {
    let mut holding = page::Holding::default();
    let captured = group
        .iter()
        .map(|&i| {
            let (request, frames) = &asked[i];
            let follows = (how.project_root, &[][..]);
            let drawing = (browsers, how.drawn, how.cancel);
            let outcome = holding
                .capture_frames(
                    how.chrome,
                    how.tools,
                    follows,
                    request,
                    frames.clone(),
                    drawing,
                )
                .map_err(|e| e.to_string());
            (i, outcome)
        })
        .collect();
    (holding, captured)
}

/// The indices of `asked`, gathered by request, each group in the order asked.
fn by_request(asked: &[(Request, Range<u64>)]) -> Vec<Vec<usize>> {
    let mut groups: Vec<(&Request, Vec<usize>)> = Vec::new();
    for (i, (request, _)) in asked.iter().enumerate() {
        match groups.iter_mut().find(|(seen, _)| *seen == request) {
            Some((_, group)) => group.push(i),
            None => groups.push((request, vec![i])),
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use scorsese_core::Fps;

    use super::*;
    use crate::Resolution;

    fn request(page: &str) -> Request {
        Request {
            page: page.to_owned(),
            resolution: Resolution::new(32, 32).expect("a legal raster"),
            fps: Fps::THIRTY,
            duration: 1.0,
            clips: BTreeMap::new(),
            words: BTreeMap::new(),
        }
    }

    #[test]
    fn clips_of_one_request_share_a_group_in_the_order_asked() {
        let asked = [
            (request("a.html"), 0..10),
            (request("b.html"), 0..10),
            (request("a.html"), 5..30),
            (request("c.html"), 0..10),
            (request("b.html"), 2..4),
        ];
        assert_eq!(by_request(&asked), [vec![0, 2], vec![1, 4], vec![3]]);
    }
}
