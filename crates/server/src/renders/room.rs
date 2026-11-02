//! What a render may take of the disk (#1647): nothing while the disk is
//! below its floor, no more than its owner's room ([`spare`],
//! [`within_quota`]), and no page capture past [`CAPTURE_BUDGET`] or past
//! that room ([`split`]).
//!
//! ## The capture budget
//!
//! A capture's frames are lossless, so a long page at a large raster could
//! fill the shared disk on its own. A render job asks for each page's frames
//! ([`super::job`]); one whose frames are expected to pass [`CAPTURE_BUDGET`],
//! or the room the job has left once the captures before it are counted, is
//! refused there, before the launcher is asked, and its clip shows its slug
//! card with the reason in the render's notes. Frames already in the page
//! cache write nothing, and cost nothing.
//!
//! **Expected** is frames × raster, a byte per [`PIXELS_PER_BYTE`] pixels: what a minute of
//! 1080p page clip measured (about 1.7 GB, `docs/web.md`, *Pages*), rounded
//! up. At 20 GB that is about ten minutes of 1080p30 in one capture.

use std::collections::HashMap;
use std::path::Path;

use scorsese_core::bytes::format_bytes;
use scorsese_render::page::{Wanted, cached_frames};

use crate::db::{self, Says, Tx, UserId};
use crate::jobs::Context;
use crate::library::{LibraryError, room};
use crate::storage::Storage;

/// The bytes a render job of `user`'s may still write: the smaller of the
/// room their quota leaves and what the disk has above its floor. Refused,
/// in plain words, when the disk has none; a member with no room left was
/// refused at ask (`request`).
pub(super) async fn spare(
    context: &Context,
    storage: &Storage,
    user: UserId,
) -> Result<u64, String> {
    let headroom = room::headroom(storage).await.map_err(said)?;
    let database = |error| said(LibraryError::Database(error));
    let mut tx = context.scoped().await.map_err(database)?;
    let used = room::kept(&mut tx, storage, user, None)
        .await
        .map_err(said)?;
    tx.commit().await.map_err(database)?;
    Ok(headroom.min(storage.quota().get().saturating_sub(used)))
}

/// [`split`] at [`CAPTURE_BUDGET`] and `spare`, for the project laid out at
/// `root`: what the page cache already holds for this server's browser costs
/// nothing, and with no browser nothing is captured, so nothing is written.
pub(super) fn capturable(
    requests: Vec<Wanted>,
    root: &Path,
    spare: u64,
) -> (Vec<Wanted>, HashMap<String, String>) {
    let chrome = crate::captures::dispatch::browser().ok();
    let cached = |wanted: &Wanted| {
        chrome.as_ref().is_none_or(|chrome| {
            cached_frames(
                root,
                &wanted.request,
                chrome.version(),
                wanted.frames.clone(),
            )
            .is_some()
        })
    };
    split(requests, CAPTURE_BUDGET, spare, cached)
}

/// A refusal from the library, said to the job's owner.
fn said(error: LibraryError) -> String {
    match error {
        LibraryError::Database(error) => db::user_facing("render job", Says::TryAgain, error),
        other => other.to_string(),
    }
}

/// Refused, in the quota's words, when `user` keeps more than their quota
/// with this render of `size` bytes already in `tx`; takes their lock.
pub(super) async fn within_quota(
    tx: &mut Tx,
    storage: &Storage,
    user: UserId,
    size: u64,
) -> Result<(), String> {
    let used = room::kept(tx, storage, user, None).await.map_err(said)?;
    let quota = storage.quota().get();
    if used > quota {
        return Err(LibraryError::Full {
            used: used.saturating_sub(size),
            quota,
            needed: size,
        }
        .to_string());
    }
    Ok(())
}

/// The most one capture is expected to write: 20 GB of frames.
const CAPTURE_BUDGET: u64 = 20 * 1000 * 1000 * 1000;

/// What a captured frame keeps per pixel of its raster, as a fraction: one
/// byte for every [`PIXELS_PER_BYTE`] pixels.
const PIXELS_PER_BYTE: u64 = 2;

/// `requests` split into those to capture and, by page path, why each of the
/// others is refused: past `budget` alone, or past what is left of `spare`
/// once the ones before it are counted. One `cached` says is already in the
/// page cache costs nothing.
pub(super) fn split(
    requests: Vec<Wanted>,
    budget: u64,
    spare: u64,
    cached: impl Fn(&Wanted) -> bool,
) -> (Vec<Wanted>, HashMap<String, String>) {
    let mut refused = HashMap::new();
    let mut left = spare;
    let kept = requests
        .into_iter()
        .filter(|wanted| {
            if cached(wanted) {
                return true;
            }
            let expected = expected(wanted);
            let why = if expected > budget {
                format!(
                    "it is too long to capture at this size: its frames would take about {}, \
                     and one capture may take {}; shorten its clip or render it smaller",
                    format_bytes(expected),
                    format_bytes(budget)
                )
            } else if expected > left {
                format!(
                    "there is no room to capture it: its frames would take about {}, and only \
                     {} is left of your storage; delete files you no longer need",
                    format_bytes(expected),
                    format_bytes(left)
                )
            } else {
                left -= expected;
                return true;
            };
            refused.insert(wanted.request.page.clone(), why);
            false
        })
        .collect();
    (kept, refused)
}

/// The bytes `wanted`'s frames are expected to take.
fn expected(wanted: &Wanted) -> u64 {
    let resolution = wanted.request.resolution;
    let pixels = u64::from(resolution.width()) * u64::from(resolution.height());
    let frames = wanted.frames.end.saturating_sub(wanted.frames.start);
    frames.saturating_mul(pixels) / PIXELS_PER_BYTE
}

#[cfg(test)]
mod tests {
    use scorsese_core::Fps;
    use scorsese_render::Resolution;
    use scorsese_render::page::Request;

    use super::*;

    fn wanted(page: &str, frames: u64) -> Wanted {
        let request = Request {
            page: page.to_owned(),
            resolution: Resolution::new(1920, 1080).unwrap(),
            fps: Fps::THIRTY,
            duration: 3600.0,
            clips: Default::default(),
            words: Default::default(),
        };
        Wanted::new(request, 0..frames)
    }

    #[test]
    fn a_capture_past_the_budget_is_refused_and_the_rest_kept() {
        // 1920×1080 at half a byte a pixel is 1,036,800 bytes a frame.
        let (kept, refused) = split(
            vec![
                wanted("pages/long.html", 19_291),
                wanted("pages/short.html", 19_290),
            ],
            CAPTURE_BUDGET,
            u64::MAX,
            |_| false,
        );

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].request.page, "pages/short.html");
        let why = &refused["pages/long.html"];
        assert!(why.contains("too long to capture"), "{why}");
    }

    #[test]
    fn captures_past_the_room_left_are_refused_once_the_ones_before_are_counted() {
        // Three of 1,036,800 bytes each, with room for two.
        let pages = || {
            vec![
                wanted("pages/a.html", 1),
                wanted("pages/b.html", 1),
                wanted("pages/c.html", 1),
            ]
        };
        let (kept, refused) = split(pages(), CAPTURE_BUDGET, 2 * 1_036_800, |_| false);

        assert_eq!(kept.len(), 2);
        assert!(refused["pages/c.html"].contains("no room to capture it"));

        // What the page cache holds already writes nothing.
        let cached = |wanted: &Wanted| wanted.request.page == "pages/a.html";
        let (kept, refused) = split(pages(), CAPTURE_BUDGET, 2 * 1_036_800, cached);
        assert_eq!(kept.len(), 3, "{refused:?}");
    }
}
