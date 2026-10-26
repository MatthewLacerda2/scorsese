//! Captioning footage whose speech was transcribed (#1549).

use super::fixture::{asked, project};
use crate::captions::caption;
use crate::words::{Word, Words};
use crate::{
    Asset, AssetId, AssetKind, Clip, ClipId, Frames, ProjectPath, Track, TrackId, TrackKind,
};

/// Footage whose speech was transcribed (#1549) is captioned like a line,
/// wherever its clip is, and footage nobody transcribed is not reported.
#[test]
fn transcribed_footage_is_captioned_and_untranscribed_footage_is_not_named() {
    let (dir, mut project) = project("footage");
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    for (id, heard) in [("sketch", true), ("b-roll", false)] {
        let path = ProjectPath::new(format!("assets/{id}.mp4"));
        if heard {
            let words = Words {
                words: vec![
                    Word {
                        text: "Bom".into(),
                        start: 0.5,
                        end: 0.8,
                    },
                    Word {
                        text: "dia.".into(),
                        start: 0.8,
                        end: 1.2,
                    },
                ],
            };
            std::fs::write(Words::transcript(&path).resolve(&dir), words.to_json()).unwrap();
        }
        project
            .assets
            .push(Asset::imported(AssetId::new(id), AssetKind::Video, path));
    }
    let mut shots = Track::new(TrackId::new("v1"), TrackKind::Video);
    shots.clips.push(Clip::new(
        ClipId::new("take"),
        AssetId::new("sketch"),
        Frames(0),
        Frames(60),
    ));
    shots.clips.push(Clip::new(
        ClipId::new("cutaway"),
        AssetId::new("b-roll"),
        Frames(60),
        Frames(60),
    ));
    project.tracks.insert(0, shots);
    let report = caption(&mut project, &dir, &asked()).unwrap();
    assert_eq!(
        report.lines,
        [(ClipId::new("take"), 1), (ClipId::new("nar"), 2)]
    );
    assert_eq!(report.untimed, [ClipId::new("nar-2")]);
    let first = project.asset(&AssetId::new("caption-take-1")).unwrap();
    assert_eq!(first.text.as_deref(), Some("Bom dia."));

    // Named, a track is captioned whatever it carries.
    let mut asked = asked();
    asked.narration = vec![TrackId::new("v1")];
    let report = caption(&mut project, &dir, &asked).unwrap();
    assert_eq!(report.lines, [(ClipId::new("take"), 1)]);
    assert_eq!(report.untimed, [ClipId::new("cutaway")]);
}
