//! What a render may take of the disk (#1647): nothing while the disk is
//! below its floor ([`on_disk`]), no more than its owner's room
//! ([`within_quota`]), and no page capture past [`CAPTURE_BUDGET`]
//! ([`split`]).
//!
//! ## The capture budget
//!
//! A capture's frames are lossless, so a long page at a large raster could
//! fill the shared disk on its own. A render job asks for each page's frames
//! ([`super::job`]); one whose frames are expected to pass [`CAPTURE_BUDGET`]
//! is refused there, before the launcher is asked, and its clip shows its slug
//! card with the reason in the render's notes.
//!
//! **Expected** is frames × raster, a byte per [`PIXELS_PER_BYTE`] pixels: what a minute of
//! 1080p page clip measured (about 1.7 GB, `docs/web.md`, *Pages*), rounded
//! up. At 20 GB that is about ten minutes of 1080p30 in one capture.

use std::collections::HashMap;

use scorsese_core::bytes::format_bytes;
use scorsese_render::page::Wanted;

use crate::db::{self, Says, Tx, UserId};
use crate::jobs::blocking;
use crate::library::{LibraryError, room};
use crate::storage::Storage;

/// Refused, in plain words, while the disk is below its floor.
pub(super) async fn on_disk(storage: &Storage) -> Result<(), String> {
    let disk = storage.clone();
    blocking("measuring the disk crashed", move || disk.on_disk())
        .await?
        .map_err(|low| low.to_string())
}

/// Refused, in the quota's words, when `user` keeps more than their quota
/// with this render of `size` bytes already in `tx`; takes their lock.
pub(super) async fn within_quota(
    tx: &mut Tx,
    storage: &Storage,
    user: UserId,
    size: u64,
) -> Result<(), String> {
    let used = room::kept(tx, storage, user, None)
        .await
        .map_err(|error| match error {
            LibraryError::Database(error) => db::user_facing("render job", Says::TryAgain, error),
            other => other.to_string(),
        })?;
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
pub(super) const CAPTURE_BUDGET: u64 = 20 * 1000 * 1000 * 1000;

/// What a captured frame keeps per pixel of its raster, as a fraction: one
/// byte for every [`PIXELS_PER_BYTE`] pixels.
const PIXELS_PER_BYTE: u64 = 2;

/// `requests` split into those within `budget` and, by page path, why each of
/// the others is refused.
pub(super) fn split(requests: Vec<Wanted>, budget: u64) -> (Vec<Wanted>, HashMap<String, String>) {
    let mut refused = HashMap::new();
    let kept = requests
        .into_iter()
        .filter(|wanted| {
            let expected = expected(wanted);
            if expected <= budget {
                return true;
            }
            refused.insert(
                wanted.request.page.clone(),
                format!(
                    "it is too long to capture at this size: its frames would take about {}, \
                     and one capture may take {}; shorten its clip or render it smaller",
                    format_bytes(expected),
                    format_bytes(budget)
                ),
            );
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
        );

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].request.page, "pages/short.html");
        let why = &refused["pages/long.html"];
        assert!(why.contains("too long to capture"), "{why}");
    }
}
