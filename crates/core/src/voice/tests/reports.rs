//! What a cut says about what it did not lay out itself.

use super::{at, project, voicing};
use crate::voice::cut_to_voice;
use crate::voice::report::under;
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Easing, Frames, Keyframe, KeyframeTrack, ProjectPath,
    PropertyPath, Track, TrackId, TrackKind,
};

fn keyed(t: u64) -> KeyframeTrack {
    let key = Keyframe {
        t: Frames(t),
        value: 1.0,
        easing: Easing::Linear,
    };
    KeyframeTrack::new(PropertyPath::new("opacity"), vec![key])
}

#[test]
fn touching_a_scene_is_not_being_under_it() {
    let spans = [(Frames(0), Frames(10)), (Frames(10), Frames(20))];
    let clip = |start, duration| {
        Clip::new(
            ClipId::new("c"),
            AssetId::new("a"),
            Frames(start),
            Frames(duration),
        )
    };
    assert_eq!(under(&clip(10, 5), &spans), [1]);
    assert_eq!(under(&clip(5, 5), &spans), [0]);
    assert_eq!(under(&clip(9, 2), &spans), [0, 1]);
    assert!(under(&clip(20, 5), &spans).is_empty());
}

#[test]
fn a_keyframe_past_a_visuals_new_end_is_reported_and_one_on_it_is_not() {
    let (dir, mut project) = project("keyed");
    project.tracks[0].clips[0].keyframes.push(keyed(51));
    project.tracks[0].clips[1].keyframes.push(keyed(100));
    let voiced = cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    assert_eq!(voiced.keyed_past_end, [ClipId::new("page-2")]);
}

#[test]
fn what_read_the_old_positions_is_named_and_nothing_else() {
    let (dir, mut project) = project("follow");
    let plain = cut_to_voice(&mut project.clone(), &dir, &voicing()).unwrap();
    assert!(plain.follow_ups.is_empty(), "{:?}", plain.follow_ups);

    let mut captions = Track::new(TrackId::new("captions"), TrackKind::Video);
    let caption = Clip::new(
        ClipId::new("caption-nar-1-1"),
        AssetId::new("still"),
        Frames(0),
        Frames(10),
    );
    captions.clips.push(caption);
    project.tracks.push(captions);
    project.tracks[3].clips[0]
        .keyframes
        .push(keyed(0).generated_by(crate::dip::TOOL));
    let recipe = ProjectPath::new("recipes/score.json");
    project.assets.push(Asset::synth(
        AssetId::new("score"),
        AssetKind::SynthAudio,
        recipe,
    ));
    project.tracks[3].clips.push(Clip::new(
        ClipId::new("cue"),
        AssetId::new("score"),
        Frames(300),
        Frames(30),
    ));
    let voiced = cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    let said = voiced.follow_ups.join("\n");
    for tool in ["caption_narration", "duck_music", "bake"] {
        assert!(said.contains(tool), "{said}");
    }
    assert!(
        voiced
            .crossed
            .iter()
            .all(|clip| clip.as_str() != "caption-nar-1-1")
    );
}

#[test]
fn lines_on_different_tracks_keep_their_whole_length() {
    let (dir, mut project) = project("apart");
    let line = project.tracks[1].clips.remove(1);
    let mut second = Track::new(TrackId::new("a2"), TrackKind::Audio);
    second.clips.push(line);
    project.tracks.push(second);
    cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    assert_eq!(at(&project, "nar-1"), ("a1".into(), 0, 90));
    assert_eq!(at(&project, "nar-2"), ("a2".into(), 51, 141));
}
