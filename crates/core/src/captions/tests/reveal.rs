//! How a caption arrives.

use super::fixture::{asked, project};
use crate::captions::{Captioning, caption};

#[test]
fn a_caption_asked_to_arrive_at_once_has_no_reveal_to_play() {
    let (dir, mut project) = project("at-once");
    let at_once = Captioning {
        arrive: 0,
        ..asked()
    };
    caption(&mut project, &dir, &at_once).unwrap();
    let clip = &project.tracks.last().unwrap().clips[0];
    let animated: Vec<&str> = clip.keyframes.iter().map(|k| k.property.as_str()).collect();
    assert_eq!(animated, ["transform.position.y"]);
}

/// A caption's id is made from its line's clip id, which may hold anything; the
/// caption's is still an asset id, and keeps the rule one does.
#[test]
fn a_line_whose_clip_id_breaks_the_id_rule_still_captions_validly() {
    let (dir, mut project) = project("odd-line");
    let line = &mut project.tracks[0].clips[0];
    assert_eq!(line.id.as_str(), "nar", "the fixture's first line");
    line.id = crate::ClipId::new("nar/open");
    caption(&mut project, &dir, &asked()).unwrap();
    assert!(project.validate().is_ok(), "{:?}", project.validate());
    let caption = &project.tracks.last().unwrap().clips[0];
    assert_eq!(caption.asset.as_str(), "caption-nar-open-1");
}
