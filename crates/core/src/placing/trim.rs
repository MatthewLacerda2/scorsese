//! Moving a placed clip, or changing what part of its source it shows.

use crate::project::Project;
use crate::time::Frames;
use crate::timeline::{Clip, ClipId};
use crate::validate::ValidationErrors;

/// Which of a placed clip's bounds to change. All three absent is
/// [`TrimError::Nothing`].
///
/// **Fields, not edges.** Each of these sets the clip field of the same name,
/// so a `start` on its own *moves* the clip and a `duration` on its own changes
/// where it ends. Reading them as the edges an editor drags would make one
/// argument mean two things depending on which others came with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Trim {
    /// Where the clip begins on the timeline.
    pub start: Option<Frames>,
    /// How long it runs.
    pub duration: Option<Frames>,
    /// Where in the source it opens, in timeline frames.
    pub source_in: Option<Frames>,
}

/// Why a clip was not trimmed. Nothing is ever partly written.
#[derive(Debug, thiserror::Error)]
pub enum TrimError {
    /// No clip in the project answers to that id.
    #[error("no clip in this project is called `{clip}`")]
    NoSuchClip {
        /// The id that was asked for.
        clip: ClipId,
    },
    /// Nothing was asked for. Reported rather than treated as a successful
    /// no-op: a caller that meant to change something and named no field has
    /// made a mistake, and a cheerful reply would hide it.
    #[error("nothing to change — say a new start, a new duration, or a new source in-point")]
    Nothing,
    /// A clip covering no frame renders nothing and is not a clip.
    #[error("a clip has to cover at least one frame")]
    Empty,
    /// The result was not a document that loads — the clip moved onto its
    /// neighbour, or trimmed past the end of the media it shows.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Changes a placed clip's bounds, and hands back the clip as it now is.
///
/// The clip stays on the track it is on. Which track a clip sits on decides
/// what is drawn over what, so moving one between tracks is a different edit
/// with a different consequence, and folding it in here would make a tool about
/// *time* quietly able to reorder the picture.
pub fn trim(project: &mut Project, id: &ClipId, bounds: &Trim) -> Result<Clip, TrimError> {
    if bounds == &Trim::default() {
        return Err(TrimError::Nothing);
    }
    if bounds.duration == Some(Frames::ZERO) {
        return Err(TrimError::Empty);
    }
    let mut proposed = project.clone();
    let mut trimmed = None;
    for track in &mut proposed.tracks {
        let Some(clip) = track.clips.iter_mut().find(|clip| &clip.id == id) else {
            continue;
        };
        if let Some(start) = bounds.start {
            clip.start = start;
        }
        if let Some(duration) = bounds.duration {
            clip.duration = duration;
        }
        if let Some(source_in) = bounds.source_in {
            clip.source_in = source_in;
        }
        trimmed = Some(clip.clone());
        // Moving a clip can carry it past its neighbour, so the order a reader
        // depends on is restored — the same reason `place` sorts.
        track.clips.sort_by_key(|clip| clip.start);
        break;
    }
    let trimmed = trimmed.ok_or_else(|| TrimError::NoSuchClip { clip: id.clone() })?;

    proposed.validate()?;
    *project = proposed;
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::super::fixture::placed;
    use super::*;

    fn shot() -> ClipId {
        ClipId::new("shot")
    }

    /// A start on its own is a move: the length and the source window are the
    /// caller's to change separately, and changing them here would make one
    /// argument mean two things.
    #[test]
    fn a_start_on_its_own_moves_the_clip_and_leaves_its_length_alone() {
        let mut project = placed();
        let clip = trim(
            &mut project,
            &shot(),
            &Trim {
                start: Some(Frames(90)),
                ..Trim::default()
            },
        )
        .expect("there is room");
        assert_eq!((clip.start, clip.duration), (Frames(90), Frames(120)));
        assert_eq!(clip.source_in, Frames::ZERO);
    }

    #[test]
    fn a_duration_on_its_own_holds_the_start_and_moves_the_end() {
        let mut project = placed();
        let clip = trim(
            &mut project,
            &shot(),
            &Trim {
                duration: Some(Frames(45)),
                ..Trim::default()
            },
        )
        .expect("a shorter window is inside the media");
        assert_eq!((clip.start, clip.duration), (Frames::ZERO, Frames(45)));
    }

    #[test]
    fn a_trim_that_asks_for_nothing_says_so() {
        let mut project = placed();
        let error = trim(&mut project, &shot(), &Trim::default()).expect_err("no field was named");
        assert!(matches!(error, TrimError::Nothing), "got {error}");
    }

    #[test]
    fn a_trim_past_the_end_of_the_media_writes_nothing() {
        let mut project = placed();
        let before = project.clone();
        let error = trim(
            &mut project,
            &shot(),
            &Trim {
                source_in: Some(Frames(30)),
                ..Trim::default()
            },
        )
        .expect_err("120 frames from frame 30 runs off the end");
        assert!(matches!(error, TrimError::Refused(_)), "got {error}");
        assert_eq!(project, before, "nothing was written");
    }

    #[test]
    fn a_clip_that_is_not_there_is_refused_by_name() {
        let mut project = placed();
        let error = trim(
            &mut project,
            &ClipId::new("nope"),
            &Trim {
                start: Some(Frames(1)),
                ..Trim::default()
            },
        )
        .expect_err("there is no such clip");
        assert!(matches!(error, TrimError::NoSuchClip { .. }), "got {error}");
    }
}
