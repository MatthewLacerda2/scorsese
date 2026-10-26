//! How a listing groups the pool: a sequence's stills under the sequence
//! (#684), each listed exactly once.

use std::path::Path;

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, HashCheck, ImageSequence, Project,
    ProjectPath, Track, TrackId, TrackKind, asset_status, listing,
};

fn image(id: &str) -> Asset {
    Asset::imported(
        AssetId::new(id),
        AssetKind::Image,
        ProjectPath::new(format!("assets/{id}.png")),
    )
}

fn sequence(id: &str, stills: &[&str]) -> Asset {
    Asset::image_sequence(
        AssetId::new(id),
        ImageSequence {
            stills: stills.iter().map(|still| AssetId::new(*still)).collect(),
            hold: Frames(1),
            looping: false,
        },
    )
}

/// A project of these assets, and (id, ids listed under it) per top-level line.
fn listed(assets: Vec<Asset>, tracks: Vec<Track>) -> Vec<(String, Vec<String>)> {
    let mut project = Project::new("listing", Fps::THIRTY);
    project.assets = assets;
    project.tracks = tracks;
    let rows = asset_status(&project, Path::new("/nowhere"), HashCheck::Skip);
    listing(&project, &rows)
        .into_iter()
        .map(|line| {
            let under = line.stills.iter().map(|s| s.id.to_string()).collect();
            (line.row.id.to_string(), under)
        })
        .collect()
}

fn owned(id: &str, under: &[&str]) -> (String, Vec<String>) {
    (
        id.to_owned(),
        under.iter().map(|s| (*s).to_owned()).collect(),
    )
}

#[test]
fn stills_sit_under_their_sequence_in_play_order_and_nowhere_else() {
    let assets = vec![
        image("b"),
        image("a"),
        image("loose"),
        sequence("spin", &["a", "b", "a"]),
    ];
    assert_eq!(
        listed(assets, vec![]),
        [owned("loose", &[]), owned("spin", &["a", "b"])],
        "a repeated still is one row, and an unowned image stays at the top"
    );
}

#[test]
fn a_still_two_sequences_play_belongs_to_the_first() {
    let assets = vec![image("a"), sequence("one", &["a"]), sequence("two", &["a"])];
    assert_eq!(
        listed(assets, vec![]),
        [owned("one", &["a"]), owned("two", &[])]
    );
}

#[test]
fn a_still_a_clip_also_uses_stays_under_its_sequence() {
    let mut track = Track::new(TrackId::new("v1"), TrackKind::Video);
    track.clips.push(Clip::new(
        ClipId::new("c"),
        AssetId::new("a"),
        Frames::ZERO,
        Frames(30),
    ));
    let mut project = Project::new("listing", Fps::THIRTY);
    project.assets = vec![image("a"), sequence("spin", &["a"])];
    project.tracks = vec![track];
    let rows = asset_status(&project, Path::new("/nowhere"), HashCheck::Skip);
    let lines = listing(&project, &rows);
    assert_eq!(lines.len(), 1, "never listed twice");
    assert_eq!(lines[0].stills[0].clip_count, 1, "and it says it is used");
    assert_eq!(lines[0].stills[0].sequence, Some(AssetId::new("spin")));
}
