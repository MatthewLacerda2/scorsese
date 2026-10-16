//! How long a placed clip runs when nobody said.

use super::PlaceError;
use crate::asset::{Asset, AssetKind};
use crate::time::{Fps, Frames};

/// How long a clip of an asset with no length of its own runs when placed
/// without a duration: five seconds, what an editor gives a still — long
/// enough to read, short enough to trim.
///
/// One number for every caller — a drop in the desktop app or on the web, and
/// `place_clip` for an assistant — so "put this title at 0:10" is the same
/// clip whoever asks.
pub const HOLD_SECONDS: f64 = 5.0;

/// How long a clip of `asset`, opening `source_in` into it, runs when the
/// placement leaves its duration out.
///
/// Three answers, by what the asset is:
///
/// - **It has a length** — measured footage or sound, a group, a sequence's one
///   pass through its stills — and the clip runs the rest of it from
///   `source_in`, or [`PlaceError::PastTheEnd`] when there is none left.
/// - **It has no length by nature** ([`AssetKind::is_timeless`]: a still, a
///   title, a colour, a page) and the clip runs [`HOLD_SECONDS`].
/// - **It has a length nobody has measured** — a file not probed yet, a shot or
///   a line not generated yet — and [`PlaceError::NoLength`] says so. Guessing
///   there makes a clip that is wrong, where five seconds of a still is only a
///   choice somebody can change.
pub fn unsaid_duration(asset: &Asset, fps: Fps, source_in: Frames) -> Result<Frames, PlaceError> {
    let length = match asset.kind {
        AssetKind::ImageSequence => asset.sequence.as_ref().map(|sequence| sequence.length()),
        _ => asset.length(fps),
    };
    if let Some(length) = length {
        return match length.get().checked_sub(source_in.get()) {
            Some(left) if left > 0 => Ok(Frames(left)),
            _ => Err(PlaceError::PastTheEnd {
                asset: asset.id.clone(),
                source_in,
                length,
            }),
        };
    }
    if asset.kind.is_timeless() {
        return Ok(fps.frames(HOLD_SECONDS));
    }
    Err(PlaceError::NoLength {
        asset: asset.id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{AssetId, ImageSequence, MediaMetadata};
    use crate::path::ProjectPath;

    fn file(kind: AssetKind) -> Asset {
        Asset::imported(AssetId::new("a"), kind, ProjectPath::new("assets/a"))
    }

    /// A still and a title have no length of their own, so leaving the
    /// duration out is a choice already made: five seconds, on the grid.
    #[test]
    fn a_still_or_a_title_runs_five_seconds() {
        let still = file(AssetKind::Image);
        let title = Asset::text(AssetId::new("t"), "hi");
        for asset in [still, title] {
            let at_thirty = unsaid_duration(&asset, Fps::THIRTY, Frames::ZERO);
            assert_eq!(at_thirty.expect("held"), Frames(150), "{}", asset.kind);
        }
        let at_24 = unsaid_duration(
            &file(AssetKind::Image),
            Fps::new(24, 1).expect("a rate"),
            Frames::ZERO,
        );
        assert_eq!(at_24.expect("held"), Frames(120));
    }

    /// A file nobody probed has a length that is not written down; inventing
    /// five seconds of it would be a clip that is simply wrong.
    #[test]
    fn an_unprobed_video_is_refused() {
        let error = unsaid_duration(&file(AssetKind::Video), Fps::THIRTY, Frames::ZERO)
            .expect_err("nobody measured it");
        assert!(matches!(error, PlaceError::NoLength { .. }), "got {error}");
    }

    #[test]
    fn measured_media_runs_the_rest_of_the_source() {
        let mut shot = file(AssetKind::Video);
        shot.media = Some(MediaMetadata {
            duration_seconds: Some(4.0),
            ..Default::default()
        });
        let rest = unsaid_duration(&shot, Fps::THIRTY, Frames(30));
        assert_eq!(rest.expect("three seconds left"), Frames(90));
    }

    /// A sequence's length follows from its stills, so it needs no probe: one
    /// pass through them, less wherever the clip opens.
    #[test]
    fn a_sequence_runs_one_pass_through_its_stills() {
        let mut sequence = ImageSequence::new(vec![AssetId::new("s"); 12]);
        sequence.hold = Frames(2);
        let asset = Asset::image_sequence(AssetId::new("spin"), sequence);
        let rest = unsaid_duration(&asset, Fps::THIRTY, Frames(4));
        assert_eq!(rest.expect("twenty frames left"), Frames(20));
    }
}
