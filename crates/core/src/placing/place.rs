//! Writing a new clip onto a track.

use crate::asset::{Asset, AssetId};
use crate::project::Project;
use crate::time::{Fps, Frames};
use crate::timeline::{Clip, ClipId, TrackId};
use crate::validate::ValidationErrors;

/// A clip to write, in frames on the project's grid.
///
/// Seconds are the caller's unit and frames are the document's; whoever builds
/// one of these has already crossed that line, so nothing below rounds anything
/// a second time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// What the clip shows, by id — the asset has to be in the table already.
    pub asset: AssetId,
    /// Which track it goes on. Must exist: creating one on a mistyped id is how
    /// a clip ends up somewhere nobody is looking.
    pub track: TrackId,
    /// Where it begins on the timeline.
    pub start: Frames,
    /// How long it runs, or `None` for *the rest of the source* — everything
    /// from [`Placement::source_in`] to the end of the measured media.
    ///
    /// Optional because "put this shot in" is a whole request on its own, and
    /// the answer is written down already: the asset's own length. An asset
    /// nobody has measured has no answer, and [`PlaceError::NoLength`] says so
    /// rather than guessing one.
    pub duration: Option<Frames>,
    /// Where in the source it opens, in **timeline** frames — see
    /// [`Clip::source_in`], which is the field this becomes.
    pub source_in: Frames,
    /// What to call it, or `None` to derive one from the asset's id and suffix
    /// it until it is free, the way an imported asset gets its own.
    pub id: Option<ClipId>,
}

/// Why a clip was not placed. Nothing is ever partly written.
#[derive(Debug, thiserror::Error)]
pub enum PlaceError {
    /// No such asset. A clip references an asset by id and only by id, so
    /// there is nothing to place.
    #[error("no asset in this project is called `{asset}`")]
    NoSuchAsset {
        /// The id that was asked for.
        asset: AssetId,
    },
    /// No such track. Refused rather than created: a track invented from a
    /// typo takes the clip with it, and a clip on a track nobody meant to have
    /// is invisible in every way except the render.
    #[error("no track in this project is called `{track}` — the tracks are {available}")]
    NoSuchTrack {
        /// The id that was asked for.
        track: TrackId,
        /// The tracks that do exist, so the next call can name one.
        available: String,
    },
    /// The chosen clip id is already in use. Clip ids are unique across the
    /// whole project, not merely across a track.
    #[error("`{id}` is already the id of a clip in this project")]
    TakenId {
        /// The id that was asked for.
        id: ClipId,
    },
    /// A duration was left out for an asset with no measured length. A still,
    /// a title, a colour and a brief nobody has generated have no length of
    /// their own; a file nobody has probed has one and it is not written down.
    #[error(
        "`{asset}` has no measured length, so how long the clip runs has to be said — \
         probe the asset, or give a duration"
    )]
    NoLength {
        /// The asset with nothing to measure against.
        asset: AssetId,
    },
    /// The source in-point is at or past the end of the media, so the rest of
    /// the source is nothing at all.
    #[error(
        "`{asset}` runs {length} and the clip would open {source_in} in, leaving nothing to show"
    )]
    PastTheEnd {
        /// The asset that ran out.
        asset: AssetId,
        /// Where the clip was asked to open.
        source_in: Frames,
        /// How much media there is.
        length: Frames,
    },
    /// A clip covering no frame renders nothing and is not a clip.
    #[error("a clip has to cover at least one frame")]
    Empty,
    /// The result was not a document that loads — an overlap with the clip
    /// already in that stretch of the track, or a window reaching past the end
    /// of the media.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// Writes a new clip onto a track, and hands back the clip as written.
///
/// What comes back carries the *resolved* values rather than the request: a
/// derived id and a duration taken from the source are decided here, and a
/// caller that could not read them back would have to guess what it just wrote.
pub fn place(project: &mut Project, placement: &Placement) -> Result<Clip, PlaceError> {
    let asset = project
        .asset(&placement.asset)
        .ok_or_else(|| PlaceError::NoSuchAsset {
            asset: placement.asset.clone(),
        })?;
    let duration = match placement.duration {
        Some(duration) => duration,
        None => remaining(asset, project.timeline_fps, placement.source_in)?,
    };
    if duration == Frames::ZERO {
        return Err(PlaceError::Empty);
    }
    let id = clip_id(project, placement)?;
    let lane = project
        .tracks
        .iter()
        .position(|track| track.id == placement.track)
        .ok_or_else(|| PlaceError::NoSuchTrack {
            track: placement.track.clone(),
            available: track_ids(project),
        })?;

    let mut proposed = project.clone();
    let mut clip = Clip::new(id, placement.asset.clone(), placement.start, duration);
    clip.source_in = placement.source_in;
    let track = &mut proposed.tracks[lane];
    track.clips.push(clip.clone());
    // `project.json` is a document people and agents read, and a track listed
    // out of order is one nobody can follow. Nothing depends on the order —
    // clips on a track cannot overlap — so this is for the reader alone.
    track.clips.sort_by_key(|clip| clip.start);

    proposed.validate()?;
    *project = proposed;
    Ok(clip)
}

/// How much source is left from `source_in` — what an omitted duration means.
fn remaining(asset: &Asset, fps: Fps, source_in: Frames) -> Result<Frames, PlaceError> {
    let Some(length) = asset.length(fps) else {
        return Err(PlaceError::NoLength {
            asset: asset.id.clone(),
        });
    };
    match length.get().checked_sub(source_in.get()) {
        Some(left) if left > 0 => Ok(Frames(left)),
        _ => Err(PlaceError::PastTheEnd {
            asset: asset.id.clone(),
            source_in,
            length,
        }),
    }
}

/// The id the new clip gets: the one asked for, if it is free, or one derived
/// from the asset's id and suffixed until it is.
fn clip_id(project: &Project, placement: &Placement) -> Result<ClipId, PlaceError> {
    let taken = |candidate: &str| {
        project
            .clips()
            .any(|(_, clip)| clip.id.as_str() == candidate)
    };
    if let Some(id) = &placement.id {
        if taken(id.as_str()) {
            return Err(PlaceError::TakenId { id: id.clone() });
        }
        return Ok(id.clone());
    }
    let base = placement.asset.as_str();
    let mut candidate = base.to_owned();
    let mut suffix = 2;
    while taken(&candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    Ok(ClipId::new(candidate))
}

/// The tracks a project has, for a refusal that has to name them.
fn track_ids(project: &Project) -> String {
    if project.tracks.is_empty() {
        return "none — this project has no tracks yet".to_owned();
    }
    project
        .tracks
        .iter()
        .map(|track| format!("`{}`", track.id))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::super::fixture::project;
    use super::*;

    fn placement() -> Placement {
        Placement {
            asset: AssetId::new("shot"),
            track: TrackId::new("v1"),
            start: Frames::ZERO,
            duration: None,
            source_in: Frames::ZERO,
            id: None,
        }
    }

    /// "Put this shot in" is a whole request, and the length is written down
    /// already — 4 seconds of media on a 30fps grid is 120 frames.
    #[test]
    fn an_omitted_duration_is_the_rest_of_the_source() {
        let mut project = project();
        let clip = place(&mut project, &placement()).expect("the shot fits");
        assert_eq!(clip.duration, Frames(120));
        assert_eq!(clip.id.as_str(), "shot", "derived from the asset");
    }

    #[test]
    fn an_omitted_duration_takes_the_source_in_point_off_the_length() {
        let mut project = project();
        let clip = place(
            &mut project,
            &Placement {
                source_in: Frames(30),
                ..placement()
            },
        )
        .expect("three seconds are left");
        assert_eq!(clip.duration, Frames(90));
    }

    /// The refusal this whole module exists for: a window reaching past the
    /// media is arithmetic that adds up and an edit that cannot be shot.
    #[test]
    fn a_window_past_the_end_of_the_media_writes_nothing() {
        let mut project = project();
        let before = project.clone();
        let error = place(
            &mut project,
            &Placement {
                duration: Some(Frames(150)),
                ..placement()
            },
        )
        .expect_err("there are only 120 frames of it");
        assert!(matches!(error, PlaceError::Refused(_)), "got {error}");
        assert_eq!(project, before, "nothing was written");
    }

    #[test]
    fn a_clip_landing_on_one_already_there_writes_nothing() {
        let mut project = project();
        place(&mut project, &placement()).expect("the first one fits");
        let before = project.clone();
        let error = place(
            &mut project,
            &Placement {
                start: Frames(60),
                duration: Some(Frames(30)),
                ..placement()
            },
        )
        .expect_err("frames 60-90 are taken");
        assert!(matches!(error, PlaceError::Refused(_)), "got {error}");
        assert_eq!(project, before, "nothing was written");
    }

    /// Opening exactly where the media ends leaves no rest of the source, and
    /// the refusal has to say *that* — "there is none of it left" and "you
    /// asked for a clip of no length" send a caller to different fixes.
    #[test]
    fn opening_at_the_very_end_of_the_media_says_the_source_ran_out() {
        let mut project = project();
        let error = place(
            &mut project,
            &Placement {
                source_in: Frames(120),
                ..placement()
            },
        )
        .expect_err("frame 120 is one past the last one");
        assert!(
            matches!(error, PlaceError::PastTheEnd { .. }),
            "got {error}"
        );
    }

    /// A second placement of the same asset is an ordinary thing to want, so
    /// the derived id gets out of its own way rather than refusing — and goes
    /// on counting up for the third.
    #[test]
    fn a_derived_id_is_suffixed_until_it_is_free() {
        let mut project = project();
        let mut at = 0;
        let mut next = || {
            at += 200;
            place(
                &mut project,
                &Placement {
                    start: Frames(at),
                    duration: Some(Frames(30)),
                    ..placement()
                },
            )
            .expect("each one fits")
            .id
        };
        assert_eq!(next().as_str(), "shot");
        assert_eq!(next().as_str(), "shot-2");
        assert_eq!(next().as_str(), "shot-3");
    }

    #[test]
    fn a_track_that_is_not_there_names_the_ones_that_are() {
        let mut project = project();
        let error = place(
            &mut project,
            &Placement {
                track: TrackId::new("v2"),
                ..placement()
            },
        )
        .expect_err("there is no v2");
        assert!(error.to_string().contains("`v1`"), "got {error}");
    }
}
