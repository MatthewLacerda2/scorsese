//! What the music ducks under: tracks of any kind, single clips, and stretches
//! of the timeline with no clip at all.

use crate::ducking::{audio_track, clip, dip, points, scored};
use scorsese_core::{
    ClipId, DuckError, Frames, Project, PropertyPath, Track, TrackId, TrackKind, Under, duck_track,
};

/// The fixture, plus a picture track whose second shot has a line spoken in
/// it — the cyberpunk teaser's arrangement.
fn with_pictures() -> Project {
    let mut project = scored(&[(60, 90)]);
    project.tracks.push(Track {
        kind: TrackKind::Video,
        ..audio_track("pic", vec![clip("p1", 0, 300), clip("p2", 300, 100)])
    });
    project
}

fn duck(project: &mut Project, under: Under) -> Result<(), DuckError> {
    duck_track(project, &under, &PropertyPath::new("volume"), dip()).map(drop)
}

fn music() -> Under {
    Under::new(TrackId::new("music"))
}

/// Naming one clip ducks under that clip alone: not the other shots on its
/// track, and not the audio tracks nobody named.
#[test]
fn a_named_clip_ducks_under_that_clip_alone() {
    let mut project = with_pictures();
    let under = Under {
        clips: vec![ClipId::new("p2")],
        ..music()
    };
    duck(&mut project, under).expect("ducks");
    assert_eq!(
        points(&project),
        vec![(291, 1.0), (300, 0.25), (400, 0.25), (418, 1.0)]
    );
}

/// A range ducks under a stretch no clip occupies: the words a page reads.
#[test]
fn a_range_ducks_where_there_is_no_clip() {
    let mut project = scored(&[]);
    let under = Under {
        ranges: vec![Frames(100)..Frames(200)],
        ..music()
    };
    duck(&mut project, under).expect("ducks");
    assert_eq!(
        points(&project),
        vec![(91, 1.0), (100, 0.25), (200, 0.25), (218, 1.0)]
    );
}

/// The ways of naming narration add up rather than replace each other.
#[test]
fn tracks_clips_and_ranges_add_up() {
    let mut project = with_pictures();
    let under = Under {
        narration: vec![TrackId::new("vo")],
        clips: vec![ClipId::new("p2")],
        ranges: vec![Frames(500)..Frames(540)],
        ..music()
    };
    duck(&mut project, under).expect("ducks");
    assert_eq!(
        points(&project).len(),
        12,
        "three dips: {:?}",
        points(&project)
    );
}

/// A video track named whole ducks under every shot on it.
#[test]
fn a_named_video_track_ducks_under_all_of_it() {
    let mut project = with_pictures();
    let under = Under {
        narration: vec![TrackId::new("pic")],
        ..music()
    };
    duck(&mut project, under).expect("ducks");
    assert_eq!(points(&project), vec![(0, 0.25), (400, 0.25), (418, 1.0)]);
}

/// A typo'd clip or track is refused, and the project is left as it was —
/// ducking under nothing and calling it done is the failure to avoid.
#[test]
fn naming_something_that_is_not_there_changes_nothing() {
    let mut project = with_pictures();
    let before = project.tracks.clone();
    let under = Under {
        narration: vec![TrackId::new("voice")],
        clips: vec![ClipId::new("p9")],
        ..music()
    };
    let error = duck(&mut project, under).expect_err("refused");
    assert_eq!(
        error,
        DuckError::Unknown(vec!["voice".to_owned(), "p9".to_owned()])
    );
    assert!(error.to_string().contains("`voice`, `p9`"), "{error}");
    assert_eq!(project.tracks, before);
}
