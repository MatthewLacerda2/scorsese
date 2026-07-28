//! Writing a proposal into the document, or refusing it.
//!
//! Refusal is the point. Dragging is the first thing in this window that can
//! corrupt an edit — two clips fighting over one instant of a track, a title
//! landing on a music track — and the document is the only model, so there is
//! nowhere for a bad state to hide until someone notices.
//!
//! The check is `scorsese-core`'s own [`Project::validate`], not a
//! reimplementation of it here. A second opinion about what is legal is a
//! second opinion that will drift, and the window would start allowing edits
//! the CLI refuses to load.

use scorsese_core::{ClipId, Project, TrackId, ValidationErrors};

use super::shape::Shape;

/// Puts `clip` on `onto` in the shape given.
///
/// All or nothing: the change is worked out on a copy, the copy is validated,
/// and only a copy that passes becomes the document. A refused drag leaves
/// `project` byte-for-byte as it was, which is what lets the gesture keep
/// running — the clip simply stops following the pointer until the pointer is
/// somewhere the clip may go.
///
/// The whole document is validated rather than the two rules a drag can break.
/// It costs a clone of a structure that is a few hundred clips at its largest,
/// and it means this window can never write a `project.json` the CLI would
/// refuse to open. The other side of that: a project already carrying a problem
/// would refuse every drag, which cannot happen today because a project only
/// opens if it validates.
pub(in crate::timeline) fn place(
    project: &mut Project,
    clip: &ClipId,
    onto: &TrackId,
    shape: Shape,
) -> Result<(), String> {
    let mut proposed = project.clone();
    relocate(&mut proposed, clip, onto, shape)?;
    proposed.validate().map_err(first_problem)?;
    *project = proposed;
    Ok(())
}

/// Lifts the clip off whatever track it is on, reshapes it, and sets it down
/// on `onto`. Both lookups happen before anything moves, so a failure here has
/// changed nothing even on the copy.
fn relocate(
    project: &mut Project,
    clip: &ClipId,
    onto: &TrackId,
    shape: Shape,
) -> Result<(), String> {
    let from = project
        .tracks
        .iter()
        .position(|track| track.clips.iter().any(|held| &held.id == clip))
        .ok_or_else(|| format!("no clip `{clip}` in this project"))?;
    let to = project
        .tracks
        .iter()
        .position(|track| &track.id == onto)
        .ok_or_else(|| format!("no track `{onto}` in this project"))?;

    let at = project.tracks[from]
        .clips
        .iter()
        .position(|held| &held.id == clip)
        .expect("the track was chosen because it holds this clip");
    let mut moved = project.tracks[from].clips.remove(at);
    moved.start = shape.start;
    moved.duration = shape.duration;
    moved.source_in = shape.source_in;

    let clips = &mut project.tracks[to].clips;
    clips.push(moved);
    // Kept in time order. Nothing reads it — clips on a track cannot overlap,
    // so their order in the array means nothing — but `project.json` is a
    // document people and agents read, and a track whose clips are listed out
    // of order is one nobody can follow. Sorted stays sorted, so this settles
    // once and then costs nothing.
    clips.sort_by_key(|clip| clip.start);
    Ok(())
}

/// The first thing validation objected to, in its own words.
///
/// One line rather than the whole report: this is shown while a drag is in
/// flight, and a drag has one thing wrong with it at a time.
fn first_problem(errors: ValidationErrors) -> String {
    errors
        .into_vec()
        .into_iter()
        .next()
        .map_or_else(|| "refused".to_owned(), |problem| problem.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{Asset, AssetId, AssetKind, Clip, Fps, Frames, Track, TrackKind};

    /// A title on `v1`, a sting on `a1`, and two clips on `v1` that touch:
    /// `head` runs 0..100 and `tail` 100..200.
    fn project() -> Project {
        let mut project = Project::new("t", Fps::THIRTY);
        project
            .assets
            .push(Asset::text(AssetId::new("title"), "hi"));
        project.assets.push(Asset::sketch(
            AssetId::new("sting"),
            AssetKind::GeneratedAudio,
            "a sting",
        ));
        let mut video = Track::new(TrackId::new("v1"), TrackKind::Video);
        video.clips.push(clip("head", "title", 0, 100));
        video.clips.push(clip("tail", "title", 100, 100));
        let mut audio = Track::new(TrackId::new("a1"), TrackKind::Audio);
        audio.clips.push(clip("hit", "sting", 400, 30));
        project.tracks.push(video);
        project.tracks.push(audio);
        project
    }

    fn clip(id: &str, asset: &str, start: u64, duration: u64) -> Clip {
        Clip::new(
            ClipId::new(id),
            AssetId::new(asset),
            Frames(start),
            Frames(duration),
        )
    }

    fn shape(start: u64, duration: u64, source_in: u64) -> Shape {
        Shape {
            start: Frames(start),
            duration: Frames(duration),
            source_in: Frames(source_in),
        }
    }

    fn find<'a>(project: &'a Project, id: &str) -> (&'a Track, &'a Clip) {
        project
            .clips()
            .find(|(_, clip)| clip.id.as_str() == id)
            .expect("the clip is in this project")
    }

    #[test]
    fn a_legal_move_lands_and_takes_its_source_offset_with_it() {
        let mut project = project();
        place(
            &mut project,
            &ClipId::new("head"),
            &TrackId::new("v1"),
            shape(220, 60, 15),
        )
        .expect("nothing is in the way at 220");
        let (track, clip) = find(&project, "head");
        assert_eq!(track.id.as_str(), "v1");
        assert_eq!(
            (clip.start, clip.duration, clip.source_in),
            (Frames(220), Frames(60), Frames(15))
        );
    }

    #[test]
    fn a_clip_may_touch_the_one_beside_it_but_not_share_a_frame_with_it() {
        let mut project = project();
        place(
            &mut project,
            &ClipId::new("head"),
            &TrackId::new("v1"),
            shape(200, 100, 0),
        )
        .expect("200 is exactly where `tail` ends");

        let refusal = place(
            &mut project,
            &ClipId::new("head"),
            &TrackId::new("v1"),
            shape(199, 100, 0),
        )
        .expect_err("one frame of overlap is an overlap");
        assert!(refusal.contains("overlap"), "{refusal}");
        assert_eq!(find(&project, "head").1.start, Frames(200), "unchanged");
    }

    #[test]
    fn picture_is_refused_on_an_audio_track_and_the_document_does_not_move() {
        let mut project = project();
        let refusal = place(
            &mut project,
            &ClipId::new("head"),
            &TrackId::new("a1"),
            shape(0, 100, 0),
        )
        .expect_err("a title is not sound");
        assert!(refusal.contains("Audio"), "{refusal}");
        assert_eq!(find(&project, "head").0.id.as_str(), "v1");
    }

    #[test]
    fn a_clip_moves_between_tracks_of_the_kind_it_belongs_to() {
        let mut project = project();
        project
            .tracks
            .push(Track::new(TrackId::new("v2"), TrackKind::Video));
        place(
            &mut project,
            &ClipId::new("head"),
            &TrackId::new("v2"),
            shape(100, 100, 0),
        )
        .expect("v2 is empty and carries picture");
        assert_eq!(find(&project, "head").0.id.as_str(), "v2");
        assert_eq!(
            project.tracks[0].clips.len(),
            1,
            "and it left the track it came from"
        );
    }

    #[test]
    fn a_clip_trimmed_to_nothing_is_refused() {
        let mut project = project();
        let refusal = place(
            &mut project,
            &ClipId::new("head"),
            &TrackId::new("v1"),
            shape(0, 0, 0),
        )
        .expect_err("a clip covering no frame renders nothing");
        assert!(refusal.contains("zero duration"), "{refusal}");
    }

    #[test]
    fn a_track_keeps_its_clips_in_time_order() {
        let mut project = project();
        place(
            &mut project,
            &ClipId::new("head"),
            &TrackId::new("v1"),
            shape(300, 100, 0),
        )
        .expect("300 is clear");
        let order: Vec<&str> = project.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.id.as_str())
            .collect();
        assert_eq!(order, ["tail", "head"]);
    }
}
