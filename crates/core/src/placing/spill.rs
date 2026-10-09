//! Writing a sound onto the first lane of a set that has room for it.

use super::place::{remaining, track_ids};
use super::{PlaceError, Placement, place};
use crate::project::Project;
use crate::timeline::{Clip, ClipId, Track, TrackId, TrackKind};

/// Writes `placement`'s clip onto the first lane of its track's **spill set**
/// with room for it, making the next lane of the set when none has, and hands
/// back the lane it landed on and the clip as written.
///
/// Sound design is many short effects that overlap — a pop per card, a whoosh
/// per cut — and clips on one track may not. Placing them one track at a time
/// means every caller writes the same lane-packing loop, or meets the overlap
/// refusal first (#972). This is that loop, once.
///
/// The set is spelled by the ids alone, so nothing in the format changes: the
/// named track `sfx` and the tracks `sfx-2`, `sfx-3`… after it. They are tried
/// in that order, base first, and a new lane takes the lowest free number and
/// sits right after the set's last lane, so the set reads as one block.
///
/// **Sound only.** A video track's position is what it is drawn over, so "any
/// free one" means nothing there: the named track has to exist and be an audio
/// track — a typo is still refused and names the tracks, as [`place`] does.
/// **All or nothing, the new lane included**, exactly as [`place`].
pub fn place_spilling(
    project: &mut Project,
    placement: &Placement,
) -> Result<(TrackId, Clip), PlaceError> {
    let base = &placement.track;
    let lane = project
        .tracks
        .iter()
        .find(|track| &track.id == base)
        .ok_or_else(|| PlaceError::NoSuchTrack {
            track: base.clone(),
            available: track_ids(project),
        })?;
    if lane.kind != TrackKind::Audio {
        return Err(PlaceError::NotSound {
            track: base.clone(),
        });
    }
    let asset = project
        .asset(&placement.asset)
        .ok_or_else(|| PlaceError::NoSuchAsset {
            asset: placement.asset.clone(),
        })?;
    let duration = match placement.duration {
        Some(duration) => duration,
        None => remaining(asset, project.timeline_fps, placement.source_in)?,
    };
    // Only where it lands is decided here; `place` still says whether it may.
    let window = Clip::new(
        ClipId::new(""),
        placement.asset.clone(),
        placement.start,
        duration,
    );
    let mut lanes: Vec<(u64, &Track)> = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .filter_map(|track| member(base, &track.id).map(|number| (number, track)))
        .collect();
    lanes.sort_by_key(|(number, _)| *number);
    let fits = |track: &Track| !track.clips.iter().any(|clip| clip.overlaps(&window));

    let mut proposed = project.clone();
    let track = match lanes.iter().find(|(_, track)| fits(track)) {
        Some((_, track)) => track.id.clone(),
        None => {
            let id = next_lane(project, base);
            let after = proposed
                .tracks
                .iter()
                .rposition(|track| member(base, &track.id).is_some())
                .expect("the base track is a member of its own set");
            proposed
                .tracks
                .insert(after + 1, Track::new(id.clone(), TrackKind::Audio));
            id
        }
    };
    let clip = place(
        &mut proposed,
        &Placement {
            track: track.clone(),
            duration: Some(duration),
            ..placement.clone()
        },
    )?;
    *project = proposed;
    Ok((track, clip))
}

/// Where `track` stands in `base`'s set: 1 for the base itself, `n` for
/// `base-n` with `n` of 2 or more, and `None` for a track outside it.
fn member(base: &TrackId, track: &TrackId) -> Option<u64> {
    if track == base {
        return Some(1);
    }
    let number = track
        .as_str()
        .strip_prefix(base.as_str())?
        .strip_prefix('-')?;
    // `sfx-02` and `sfx-+3` are someone else's names, not lanes of this set.
    if !number.bytes().all(|byte| byte.is_ascii_digit()) || number.starts_with('0') {
        return None;
    }
    number.parse().ok().filter(|number| *number >= 2)
}

/// The lowest `base-n` no track anywhere in the project is called.
fn next_lane(project: &Project, base: &TrackId) -> TrackId {
    (2..)
        .map(|number| TrackId::new(format!("{base}-{number}")))
        .find(|candidate| !project.every_track().any(|track| &track.id == candidate))
        .expect("a project has finitely many tracks")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetId;
    use crate::time::Frames;

    /// A one-second pop (30 frames at 30fps), an audio track `sfx` and a
    /// music track after it.
    fn project() -> Project {
        Project::from_json(
            r#"{
              "schema_version": 47,
              "name": "T",
              "timeline_fps": { "num": 30, "den": 1 },
              "assets": [
                { "id": "pop", "kind": "audio", "path": "assets/pop.wav",
                  "media": { "duration_seconds": 1.0 } }
              ],
              "tracks": [
                { "id": "sfx", "kind": "audio", "clips": [] },
                { "id": "music", "kind": "audio", "clips": [] },
                { "id": "v1", "kind": "video", "clips": [] }
              ]
            }"#,
        )
        .expect("the fixture is a project")
    }

    fn pop(track: &str, start: u64) -> Placement {
        Placement {
            asset: AssetId::new("pop"),
            track: TrackId::new(track),
            start: Frames(start),
            duration: None,
            source_in: Frames::ZERO,
            id: None,
        }
    }

    fn lane(project: &mut Project, start: u64) -> String {
        place_spilling(project, &pop("sfx", start))
            .expect("there is always a lane")
            .0
            .to_string()
    }

    /// Overlapping pops stack onto new lanes; one that fits on the base again
    /// goes back there rather than making another.
    #[test]
    fn an_overlapping_sound_spills_onto_the_next_lane() {
        let mut project = project();
        assert_eq!(lane(&mut project, 0), "sfx");
        assert_eq!(lane(&mut project, 10), "sfx-2");
        assert_eq!(lane(&mut project, 20), "sfx-3");
        assert_eq!(lane(&mut project, 35), "sfx", "frames 30-60 are free there");
        assert_eq!(lane(&mut project, 45), "sfx-2");
        let ids: Vec<_> = project.tracks.iter().map(|t| t.id.to_string()).collect();
        assert_eq!(
            ids,
            ["sfx", "sfx-2", "sfx-3", "music", "v1"],
            "kept together"
        );
    }

    /// Only `sfx-<n>` with a plain number is a lane of `sfx`; and a lane of
    /// the other kind is passed over, its id still not reused.
    #[test]
    fn only_the_numbered_audio_lanes_belong_to_the_set() {
        let mut project = project();
        for (id, kind) in [
            ("sfx-02", TrackKind::Audio),
            ("sfxy-2", TrackKind::Audio),
            ("sfx-2", TrackKind::Video),
        ] {
            project.tracks.push(Track::new(TrackId::new(id), kind));
        }
        assert_eq!(lane(&mut project, 0), "sfx");
        assert_eq!(lane(&mut project, 0), "sfx-3");
        assert!(project.tracks.iter().take(5).all(|t| t.clips.len() <= 1));
    }

    #[test]
    fn a_picture_track_or_a_missing_one_is_refused_and_nothing_is_written() {
        let mut project = project();
        let before = project.clone();
        let video = place_spilling(&mut project, &pop("v1", 0)).expect_err("v1 is video");
        assert!(matches!(video, PlaceError::NotSound { .. }), "got {video}");
        let missing = place_spilling(&mut project, &pop("sfz", 0)).expect_err("a typo");
        assert!(missing.to_string().contains("`sfx`"), "got {missing}");
        assert_eq!(project, before);
    }

    /// The new lane and the clip are one edit.
    #[test]
    fn a_refused_clip_adds_no_lane() {
        let mut project = project();
        lane(&mut project, 0);
        let before = project.clone();
        let error = place_spilling(
            &mut project,
            &Placement {
                duration: Some(Frames(90)),
                ..pop("sfx", 0)
            },
        )
        .expect_err("the pop is one second long");
        assert!(matches!(error, PlaceError::Refused(_)), "got {error}");
        assert_eq!(project, before);
    }
}
