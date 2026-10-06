//! Writing a new clip onto a lane made for it.

use super::{PlaceError, Placement, place};
use crate::asset::AssetId;
use crate::authoring::numbered;
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{Clip, Track, TrackId, TrackKind};

/// Writes a clip of `asset` at `start` onto a new track of the kind it needs,
/// and hands back the new track's id and the clip as written.
///
/// What a hand dropping an asset wants when no lane under it can take it — the
/// timeline is empty, the drop is below the last lane, or the lane there
/// carries the other kind. Needing a sound lane before a sound can be used is
/// exactly the knowledge nobody with an idea and some footage should need.
///
/// The lane is the one `track_new` makes — numbered `v…`/`a…` from the lowest
/// free number, appended, so a new video track goes **on top** — and its kind
/// is [`TrackKind::taking`], the rule validation holds the clip to. The clip is
/// [`place`]'s: a derived id, `source_in` zero, `duration` `None` for the rest
/// of the source. **All or nothing, the lane included**: a clip that cannot be
/// placed leaves no empty lane behind.
///
/// [`place`] itself is untouched — a caller that names a track still gets a
/// refusal for a track that is not there, never a new one.
pub fn place_on_new_track(
    project: &mut Project,
    asset: &AssetId,
    start: Frames,
    duration: Option<Frames>,
) -> Result<(TrackId, Clip), PlaceError> {
    let kind = project
        .asset(asset)
        .ok_or_else(|| PlaceError::NoSuchAsset {
            asset: asset.clone(),
        })?
        .kind;
    let lane = TrackKind::taking(kind).ok_or_else(|| PlaceError::NoTrackKind {
        asset: asset.clone(),
    })?;
    let mut proposed = project.clone();
    let track = numbered(&proposed, lane);
    proposed.tracks.push(Track::new(track.clone(), lane));
    let placement = Placement {
        asset: asset.clone(),
        track: track.clone(),
        start,
        duration,
        source_in: Frames::ZERO,
        id: None,
    };
    let clip = place(&mut proposed, &placement)?;
    *project = proposed;
    Ok((track, clip))
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{placed, project};
    use super::*;
    use crate::asset::{Asset, AssetKind};
    use crate::path::ProjectPath;

    /// The fixture, with a sound added beside its shot.
    fn with_music() -> Project {
        let mut project = placed();
        project.assets.push(Asset::imported(
            AssetId::new("music"),
            AssetKind::Audio,
            ProjectPath::new("assets/m.wav"),
        ));
        project
    }

    /// A sound needs a sound lane, and nobody had to make it first.
    #[test]
    fn a_sound_gets_an_audio_lane_of_its_own() {
        let mut project = with_music();
        let (track, clip) = place_on_new_track(
            &mut project,
            &AssetId::new("music"),
            Frames(30),
            Some(Frames(60)),
        )
        .expect("an audio lane takes a sound");
        assert_eq!(track.as_str(), "a1");
        let lane = project.tracks.last().expect("a lane was added");
        assert_eq!((lane.id.clone(), lane.kind), (track, TrackKind::Audio));
        assert_eq!(lane.clips, vec![clip]);
    }

    /// A second picture lane is numbered past the first and appended — on
    /// top, as `track_new` puts it.
    #[test]
    fn a_picture_gets_a_video_lane_on_top() {
        let mut project = placed();
        let (track, clip) =
            place_on_new_track(&mut project, &AssetId::new("shot"), Frames::ZERO, None)
                .expect("the new lane is empty");
        assert_eq!(track.as_str(), "v2");
        assert_eq!(project.tracks.len(), 2);
        assert_eq!(project.tracks[1].kind, TrackKind::Video);
        assert_eq!(clip.duration, Frames(120), "the rest of the source");
        assert_eq!(clip.id.as_str(), "shot-2");
    }

    /// The lane and the clip are one edit: a clip that cannot be placed
    /// leaves no empty lane behind.
    #[test]
    fn a_refused_clip_adds_no_lane() {
        let mut project = project();
        let before = project.clone();
        let error = place_on_new_track(
            &mut project,
            &AssetId::new("shot"),
            Frames::ZERO,
            Some(Frames(500)),
        )
        .expect_err("there are only 120 frames of it");
        assert!(matches!(error, PlaceError::Refused(_)), "got {error}");
        assert_eq!(project, before);
        let missing = place_on_new_track(&mut project, &AssetId::new("nope"), Frames::ZERO, None);
        assert!(matches!(missing, Err(PlaceError::NoSuchAsset { .. })));
    }

    /// Every kind there is has a lane to go to, and it is the one validation
    /// accepts — so a drop never meets [`PlaceError::NoTrackKind`] today.
    #[test]
    fn every_kind_has_a_lane() {
        use AssetKind as K;
        let kinds = [
            K::Video,
            K::Image,
            K::Audio,
            K::Text,
            K::Color,
            K::Shape,
            K::Icon,
            K::ImageSequence,
            K::Html,
            K::GeneratedVideo,
            K::GeneratedImage,
            K::GeneratedAudio,
            K::SynthAudio,
            K::Group,
        ];
        for kind in kinds {
            let lane = TrackKind::taking(kind).expect("every kind has a lane");
            assert_eq!(lane == TrackKind::Audio, kind.is_audible(), "{kind:?}");
        }
    }
}
