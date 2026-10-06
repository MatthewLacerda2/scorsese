//! Moving a placed clip onto another track.

use crate::project::Project;
use crate::timeline::{Clip, ClipId, TrackId};
use crate::validate::ValidationErrors;

use super::Trim;
use super::place::track_ids;

/// Where a clip is moved to: a track, and optionally new bounds on it.
///
/// **One edit, not two.** A clip dragged down a lane and along it in the same
/// gesture lands at a new start on a new track, and doing that as a relocation
/// followed by a trim would pass through a document nobody asked for — the clip
/// at its old start on the new track — which can be refused for an overlap the
/// finished edit does not have. So the bounds travel with the track: a start,
/// and for a caller that also shortens the clip to fit the gap it lands in, a
/// duration and an in-point too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// The track it goes onto. Must exist and carry the clip's kind: picture
    /// onto a video track, sound onto an audio one.
    pub track: TrackId,
    /// The bounds it has there, each field set as [`trim`](super::trim) sets
    /// it. All absent keeps the ones it has — a clip lifted straight up or down
    /// onto the lane above or below.
    pub bounds: Trim,
}

/// Why a clip was not moved. Nothing is ever partly written.
#[derive(Debug, thiserror::Error)]
pub enum RelocateError {
    /// No clip in the project answers to that id.
    #[error("no clip in this project is called `{clip}`")]
    NoSuchClip {
        /// The id that was asked for.
        clip: ClipId,
    },
    /// No such track. Refused rather than created, for [`place`](super::place)'s
    /// reason: a track invented from a typo takes the clip with it.
    #[error("no track in this project is called `{track}` — the tracks are {available}")]
    NoSuchTrack {
        /// The id that was asked for.
        track: TrackId,
        /// The tracks that do exist, so the next call can name one.
        available: String,
    },
    /// The clip is on that track already and no new bounds were given, so
    /// nothing would change. Reported rather than treated as a successful
    /// no-op, as [`TrimError::Nothing`](super::TrimError::Nothing) is.
    #[error(
        "`{clip}` is on `{track}` already — name another track, or give a start, duration or source in-point"
    )]
    AlreadyThere {
        /// The clip asked about.
        clip: ClipId,
        /// The track it is on.
        track: TrackId,
    },
    /// The result was not a document that loads — picture onto a sound track
    /// or the other way round, a clip landing on one already there, or an
    /// arrow left attached to a clip that no longer has a place on screen.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Lifts a placed clip off its track and sets it down on another, and hands
/// back the clip as it now is.
///
/// Everything about the clip but the bounds asked for goes with it — its id,
/// its keyframes, its speed — because what changes is *where* it is, not
/// *what* it is. An arrow attached to it follows it by that id.
///
/// This is the edit [`trim`](super::trim) refuses to be. Which track a picture
/// sits on decides what is drawn over what, so moving one between tracks can
/// change the frame even when nothing moves in time.
pub fn relocate(
    project: &mut Project,
    id: &ClipId,
    to: &Relocation,
) -> Result<Clip, RelocateError> {
    let from = project
        .tracks
        .iter()
        .position(|track| track.clips.iter().any(|clip| &clip.id == id))
        .ok_or_else(|| RelocateError::NoSuchClip { clip: id.clone() })?;
    let onto = project
        .tracks
        .iter()
        .position(|track| track.id == to.track)
        .ok_or_else(|| RelocateError::NoSuchTrack {
            track: to.track.clone(),
            available: track_ids(project),
        })?;
    if from == onto && to.bounds == Trim::default() {
        return Err(RelocateError::AlreadyThere {
            clip: id.clone(),
            track: to.track.clone(),
        });
    }

    let mut proposed = project.clone();
    let clips = &mut proposed.tracks[from].clips;
    let at = clips
        .iter()
        .position(|clip| &clip.id == id)
        .expect("the track was chosen because it holds this clip");
    let mut moved = clips.remove(at);
    to.bounds.apply(&mut moved);
    let landed = &mut proposed.tracks[onto].clips;
    landed.push(moved.clone());
    // Kept in time order for the reader, as `place` and `trim` keep it.
    landed.sort_by_key(|clip| clip.start);

    proposed.validate()?;
    *project = proposed;
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::super::fixture::placed;
    use super::*;
    use crate::time::Frames;
    use crate::timeline::{Track, TrackKind};

    fn shot() -> ClipId {
        ClipId::new("shot")
    }

    /// `placed()` with a second video track and an audio one beside it.
    fn with_lanes() -> Project {
        let mut project = placed();
        project
            .tracks
            .push(Track::new(TrackId::new("v2"), TrackKind::Video));
        project
            .tracks
            .push(Track::new(TrackId::new("a1"), TrackKind::Audio));
        project
    }

    fn onto(track: &str, start: Option<u64>) -> Relocation {
        Relocation {
            track: TrackId::new(track),
            bounds: Trim {
                start: start.map(Frames),
                ..Trim::default()
            },
        }
    }

    #[test]
    fn a_clip_moves_onto_another_track_keeping_everything_but_where_it_is() {
        let mut project = with_lanes();
        let clip = relocate(&mut project, &shot(), &onto("v2", None)).expect("v2 is empty");
        assert_eq!((clip.start, clip.duration), (Frames::ZERO, Frames(120)));
        let (track, _) = project
            .clips()
            .find(|(_, clip)| clip.id == shot())
            .expect("it is still in the project");
        assert_eq!(track.id.as_str(), "v2");
        assert!(project.tracks[0].clips.is_empty(), "and it left v1");
    }

    #[test]
    fn a_start_travels_with_the_track_as_one_edit() {
        let mut project = with_lanes();
        let clip = relocate(&mut project, &shot(), &onto("v2", Some(90))).expect("v2 is empty");
        assert_eq!(clip.start, Frames(90));
    }

    /// Landing in a gap shorter than the clip is one edit: shortened on the
    /// way, it never passes through a document where it overlaps.
    #[test]
    fn a_clip_shortened_on_the_way_fits_a_gap_it_would_not_fit_whole() {
        let mut project = with_lanes();
        relocate(&mut project, &shot(), &onto("v2", Some(60))).expect("v2 is empty");
        super::super::place(
            &mut project,
            &super::super::Placement {
                asset: crate::asset::AssetId::new("shot"),
                track: TrackId::new("v1"),
                start: Frames::ZERO,
                duration: Some(Frames(30)),
                source_in: Frames::ZERO,
                id: Some(ClipId::new("short")),
            },
        )
        .expect("v1 is empty again");
        let whole = relocate(&mut project, &shot(), &onto("v1", Some(10)));
        assert!(whole.is_err(), "120 frames from 10 run into `short`");
        let to = Relocation {
            track: TrackId::new("v1"),
            bounds: Trim {
                start: Some(Frames(30)),
                duration: Some(Frames(40)),
                source_in: Some(Frames(15)),
            },
        };
        let clip = relocate(&mut project, &shot(), &to).expect("30..70 is free");
        assert_eq!(
            (clip.start, clip.duration, clip.source_in),
            (Frames(30), Frames(40), Frames(15))
        );
    }

    /// The refusal `place` gives for the same mistake: picture has no business
    /// on a track that is only ever mixed.
    #[test]
    fn picture_onto_a_sound_track_writes_nothing() {
        let mut project = with_lanes();
        let before = project.clone();
        let error = relocate(&mut project, &shot(), &onto("a1", None))
            .expect_err("a video clip is not sound");
        assert!(matches!(error, RelocateError::Refused(_)), "got {error}");
        assert_eq!(project, before, "nothing was written");
    }

    #[test]
    fn landing_on_a_clip_already_there_writes_nothing() {
        let mut project = with_lanes();
        relocate(&mut project, &shot(), &onto("v2", None)).expect("v2 is empty");
        super::super::place(
            &mut project,
            &super::super::Placement {
                asset: crate::asset::AssetId::new("shot"),
                track: TrackId::new("v1"),
                start: Frames::ZERO,
                duration: None,
                source_in: Frames::ZERO,
                id: Some(ClipId::new("other")),
            },
        )
        .expect("v1 is empty again");
        let before = project.clone();
        let error = relocate(&mut project, &ClipId::new("other"), &onto("v2", Some(60)))
            .expect_err("frames 60-120 of v2 are taken");
        assert!(matches!(error, RelocateError::Refused(_)), "got {error}");
        assert_eq!(project, before, "nothing was written");
    }

    #[test]
    fn the_same_track_with_no_start_is_nothing_to_do_and_says_so() {
        let mut project = with_lanes();
        let error =
            relocate(&mut project, &shot(), &onto("v1", None)).expect_err("it is on v1 already");
        assert!(
            matches!(error, RelocateError::AlreadyThere { .. }),
            "got {error}"
        );
    }

    #[test]
    fn a_track_that_is_not_there_names_the_ones_that_are() {
        let mut project = with_lanes();
        let error = relocate(&mut project, &shot(), &onto("v9", None)).expect_err("no v9");
        assert!(error.to_string().contains("`v2`"), "got {error}");
    }
}
