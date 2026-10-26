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
