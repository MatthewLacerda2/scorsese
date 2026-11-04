//! The page look (#1550): one page per caption track, played by one clip.

use super::fixture::{asked, project};
use crate::captions::{CaptionError, Captioning, Chunk, Chunking, Look, caption};
use crate::{AssetId, AssetKind, Clip, ClipId, Frames};

/// A page that lists each caption's words by the names `scorsese.words`
/// gives them, one caption to a line.
fn listed(_: &Captioning, chunks: &[Chunk]) -> String {
    chunks
        .iter()
        .map(|chunk| {
            chunk
                .words
                .iter()
                .map(|word| format!("{}/{}", chunk.line, word.name))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn paged() -> Captioning {
    Captioning {
        chunking: Chunking::WORDS,
        look: Look::Page(listed),
        ..asked()
    }
}

#[test]
fn the_captions_are_one_page_played_across_them() {
    let (dir, mut project) = project("page");
    let report = caption(&mut project, &dir, &paged()).unwrap();
    assert_eq!(report.page, Some(AssetId::new("caption-page-captions")));

    let page = project
        .asset(&AssetId::new("caption-page-captions"))
        .unwrap();
    assert_eq!(page.kind, AssetKind::Html);
    let path = page.path.clone().unwrap();
    assert_eq!(path.as_str(), "pages/caption-page-captions.html");
    let html = std::fs::read_to_string(path.resolve(&dir).unwrap()).unwrap();
    assert_eq!(
        html.lines().next(),
        Some("nar/o nar/problema nar/não"),
        "{html}"
    );
    assert!(page.note.is_some(), "it says who wrote it");

    let track = project.tracks.last().unwrap();
    assert_eq!(track.clips.len(), 1, "{:?}", track.clips);
    let clip = &track.clips[0];
    assert_eq!(clip.id.as_str(), "caption-page-captions");
    // From "O" (1.52 s, less the 0.07 s lead: frame 44) to 0.35 s past
    // "eles." (4.90 s): 5.25 s, frame 158 (157.5 rounds up).
    assert_eq!((clip.start.get(), clip.end().get()), (44, 158));
    assert!(project.validate().is_ok(), "{:?}", project.validate());
}

#[test]
fn a_rerun_rewrites_the_same_page_and_replaces_its_clip() {
    let (dir, mut project) = project("page-rerun");
    caption(&mut project, &dir, &paged()).unwrap();
    project.tracks[0].clips[0].start = Frames(60);
    let report = caption(&mut project, &dir, &paged()).unwrap();
    assert_eq!(report.replaced, 1);
    let pages: Vec<_> = project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Html)
        .collect();
    assert_eq!(pages.len(), 1, "no second page beside the first");
    assert_eq!(
        std::fs::read_dir(dir.join("pages")).unwrap().count(),
        1,
        "no suffixed copy of the file"
    );
    let clip = &project.tracks.last().unwrap().clips[0];
    assert_eq!(clip.start.get(), 74, "moved with the line");
}

#[test]
fn the_text_look_after_the_page_takes_the_page_clip_away() {
    let (dir, mut project) = project("page-then-text");
    caption(&mut project, &dir, &paged()).unwrap();
    caption(&mut project, &dir, &asked()).unwrap();
    let ids: Vec<&str> = project
        .tracks
        .last()
        .unwrap()
        .clips
        .iter()
        .map(|clip| clip.asset.as_str())
        .collect();
    assert_eq!(ids, ["caption-nar-1", "caption-nar-2"]);
}

#[test]
fn a_page_id_held_by_something_else_is_refused_and_nothing_is_written() {
    let (dir, mut project) = project("page-taken");
    project.tracks[0].clips.push(Clip::new(
        ClipId::new("caption-page-captions"),
        AssetId::new("vo-2"),
        Frames(400),
        Frames(10),
    ));
    let before = project.clone();
    let refused = caption(&mut project, &dir, &paged()).unwrap_err();
    assert_eq!(
        refused,
        CaptionError::Taken("caption-page-captions".to_owned())
    );
    assert_eq!(project, before);
    assert!(!dir.join("pages").exists());
}
