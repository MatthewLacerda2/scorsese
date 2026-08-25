//! Adding a lane to the timeline.

use super::AuthorError;
use crate::project::Project;
use crate::timeline::{Track, TrackId, TrackKind};

/// A lane to add: what it carries, and what it is called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lane {
    /// Picture or sound. It is what decides which assets may sit here, so
    /// there is no default for it and no constructor that picks one.
    pub kind: TrackKind,
    /// What to call it, or `None` to number it — `v1`, `v2`, `a1`, `a2` — from
    /// the lowest free number for its kind.
    pub id: Option<TrackId>,
    /// What a human calls the lane. Cosmetic, and absent is not a fault.
    pub name: Option<String>,
    /// Why this lane is here, for whoever reads the project next. Never
    /// rendered — see [`Track::note`].
    pub note: Option<String>,
}

impl Lane {
    /// An unnamed lane of the given kind, numbered when it is added.
    pub fn of(kind: TrackKind) -> Self {
        Self {
            kind,
            id: None,
            name: None,
            note: None,
        }
    }
}

/// Adds an empty track, and hands back the id it got.
///
/// **Appended, which for video means on top.** Video tracks composite in array
/// order, so the newest lane is drawn over everything already there — which is
/// what a caption over footage wants, and the reason the position is not an
/// argument. Reordering the layers is a different edit and stays a
/// `project_write`.
pub fn add_track(project: &mut Project, lane: &Lane) -> Result<TrackId, AuthorError> {
    let id = match &lane.id {
        Some(asked) => {
            if project.tracks.iter().any(|track| &track.id == asked) {
                return Err(AuthorError::TakenTrackId { id: asked.clone() });
            }
            asked.clone()
        }
        None => numbered(project, lane.kind),
    };

    let mut proposed = project.clone();
    let mut track = Track::new(id.clone(), lane.kind);
    track.name.clone_from(&lane.name);
    track.note.clone_from(&lane.note);
    proposed.tracks.push(track);
    proposed.validate()?;
    *project = proposed;
    Ok(id)
}

/// The lowest free `v`/`a` number for this kind — the names an editor's lanes
/// have had since long before this one.
///
/// **Counted to a bound rather than searched until something gives.** `n`
/// tracks cannot take all of `n + 1` numbers, so one of `1..=n+1` is always
/// free and the range is a proof rather than a guess. A bare `loop` would find
/// the same id and would also be a naming rule that *can* spin — the mutation
/// signal hangs on it rather than failing, which is what a real spin would do
/// to a test run.
fn numbered(project: &Project, kind: TrackKind) -> TrackId {
    let letter = match kind {
        TrackKind::Video => 'v',
        TrackKind::Audio => 'a',
    };
    (1..=project.tracks.len() + 1)
        .map(|number| TrackId::new(format!("{letter}{number}")))
        .find(|candidate| !project.tracks.iter().any(|track| &track.id == candidate))
        .expect("n tracks cannot take every one of n + 1 numbers")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Fps;

    fn project() -> Project {
        Project::new("T", Fps::new(30, 1).expect("30fps"))
    }

    #[test]
    fn lanes_are_numbered_by_kind_from_the_lowest_free_number() {
        let mut project = project();
        let first = add_track(&mut project, &Lane::of(TrackKind::Video)).expect("a video lane");
        let sound = add_track(&mut project, &Lane::of(TrackKind::Audio)).expect("an audio lane");
        let second =
            add_track(&mut project, &Lane::of(TrackKind::Video)).expect("another video lane");
        assert_eq!(
            (first.as_str(), sound.as_str(), second.as_str()),
            ("v1", "a1", "v2")
        );
    }

    /// A new video lane is drawn over the ones already there, which is what
    /// makes it the answer to two clips wanting one stretch of a track.
    #[test]
    fn a_new_video_lane_goes_on_top() {
        let mut project = project();
        add_track(&mut project, &Lane::of(TrackKind::Video)).expect("under");
        let over = add_track(&mut project, &Lane::of(TrackKind::Video)).expect("over");
        assert_eq!(project.tracks.last().expect("two lanes").id, over);
    }

    /// The bound the numbering counts to has to clear the ids already taken:
    /// a lane named `v1` by hand is one of them, so the next automatic video
    /// lane is `v2` and not a second `v1` — and with three lanes filed under
    /// the first three numbers, the search still reaches a free fourth.
    #[test]
    fn a_hand_named_lane_is_counted_among_the_numbers_taken() {
        let mut project = project();
        for name in ["v1", "v2", "v3"] {
            add_track(
                &mut project,
                &Lane {
                    id: Some(TrackId::new(name)),
                    ..Lane::of(TrackKind::Video)
                },
            )
            .expect("a hand-named lane");
        }
        let next = add_track(&mut project, &Lane::of(TrackKind::Video)).expect("the next one");
        assert_eq!(next.as_str(), "v4");
    }

    #[test]
    fn an_id_already_in_use_is_refused_and_adds_nothing() {
        let mut project = project();
        add_track(&mut project, &Lane::of(TrackKind::Video)).expect("a video lane");
        let again = add_track(
            &mut project,
            &Lane {
                id: Some(TrackId::new("v1")),
                ..Lane::of(TrackKind::Video)
            },
        );
        assert!(matches!(again, Err(AuthorError::TakenTrackId { .. })));
        assert_eq!(project.tracks.len(), 1);
    }
}
