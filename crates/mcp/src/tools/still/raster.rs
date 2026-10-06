//! Which raster `still` composites at: a size the caller named, or a preview
//! size in the shape the caller or the edit asks for (#827).
//!
//! The default used to be one landscape raster, and a vertical edit had to pass
//! `resolution` to see its own shape — and the one portrait size a model has to
//! hand is the delivery's. The 2026-10-05 9:16 video asked for every one of its
//! 23 stills at 1080x1920, more than twice the pixels of the default, and those
//! pictures were most of what the assistant re-read on every later call. So the
//! preview size is a **pixel budget** rather than a raster, and the shape it is
//! spent in comes from the caller (`"9:16"`) or, when nobody says, from the
//! edit.
//!
//! A project has no shape of its own — render settings are chosen per render —
//! so the edit's shape is read off what it already records, never invented:
//! the first picture on the timeline whose size or aspect is a fact (a probed
//! file, a generated shot's or still's requested aspect). That is a guess about
//! the delivery, and the reply says what it was taken from, so a caller cutting
//! landscape footage into a vertical film knows to say `"9:16"`.

use scorsese_core::{Asset, AssetKind, Project, TrackKind};
use scorsese_render::Resolution;

/// The preview's pixel budget: 1280x720's area, the landscape default this
/// replaces, so a landscape edit's default is unchanged.
pub(super) const FRAME: u64 = 1280 * 720;

/// A sheet cell's budget: a quarter of [`FRAME`], so five cells together cost
/// about one frame and a title in one is still read rather than guessed at.
pub(super) const CELL: u64 = 640 * 360;

/// How far from square a preview may be, either way up. A strip of a source
/// (a 40x1000 ribbon cut into the edit first) would otherwise make the whole
/// preview a ribbon; no delivery is narrower than this.
const MOST: f64 = 4.0;

/// The raster to composite at, and — when the edit chose its shape — the
/// asset it was read from, for the reply to name.
pub(super) fn choose<'a>(
    asked: Option<&str>,
    project: &'a Project,
    budget: u64,
) -> Result<(Resolution, Option<&'a str>), String> {
    match asked {
        Some(text) if text.contains(':') => Ok((sized(aspect(text)?, budget)?, None)),
        Some(text) => text
            .parse()
            .map(|resolution| (resolution, None))
            .map_err(|problem| format!("resolution: {problem}")),
        None => match shape(project) {
            Some((ratio, from)) => Ok((sized(ratio, budget)?, Some(from))),
            None => Ok((sized((16, 9), budget)?, None)),
        },
    }
}

/// `9:16` as its two terms, refusing anything that is not two positive
/// whole numbers.
fn aspect(text: &str) -> Result<(u32, u32), String> {
    let refused = || {
        format!(
            "resolution: {text} is not an aspect — write one like 9:16, or a raster like 720x1280"
        )
    };
    let (across, down) = text.split_once(':').ok_or_else(refused)?;
    let term = |part: &str| part.trim().parse::<u32>().ok().filter(|&n| n > 0);
    Ok((
        term(across).ok_or_else(refused)?,
        term(down).ok_or_else(refused)?,
    ))
}

/// The raster of `budget` pixels in the shape `across:down`, each side
/// rounded to the even number the encoder's chroma needs.
fn sized((across, down): (u32, u32), budget: u64) -> Result<Resolution, String> {
    let ratio = (f64::from(across) / f64::from(down)).clamp(1.0 / MOST, MOST);
    // A budget under a million pixels is exact in an f64.
    let width = (budget as f64 * ratio).sqrt();
    let even = |side: f64| {
        // A positive side of a few thousand pixels at most.
        let halves = (side / 2.0).round() as u32;
        halves.max(1) * 2
    };
    Resolution::new(even(width), even(width / ratio)).map_err(|problem| format!("{problem}"))
}

/// The edit's shape and the asset it was read from: the first clip, in time,
/// on the lowest video track that has one whose shape is known.
///
/// The lowest track first because it is the bottom layer, which is what
/// fills the frame; a logo on a track above says nothing about the frame it
/// sits in.
fn shape(project: &Project) -> Option<((u32, u32), &str)> {
    project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Video)
        .find_map(|track| {
            let mut clips: Vec<_> = track.clips.iter().collect();
            clips.sort_by_key(|clip| clip.start);
            clips.into_iter().find_map(|clip| {
                let asset = project.asset(&clip.asset)?;
                Some((of(asset)?, asset.id.as_str()))
            })
        })
}

/// One asset's shape, when it is a fact: what a probe measured, or the
/// aspect a generated picture was asked for. Inline kinds — a title, a
/// colour, a page — take whatever shape the render is, so they have none.
fn of(asset: &Asset) -> Option<(u32, u32)> {
    if let Some(media) = &asset.media
        && let (Some(width), Some(height)) = (media.width, media.height)
        && width > 0
        && height > 0
    {
        return Some((width, height));
    }
    let spelled = match asset.kind {
        AssetKind::GeneratedVideo => asset.video_request().aspect.as_str(),
        AssetKind::GeneratedImage => asset.image_request().aspect.as_str(),
        _ => return None,
    };
    aspect(spelled).ok()
}

#[cfg(test)]
mod tests;
