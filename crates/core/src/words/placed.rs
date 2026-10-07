//! A line's words where the timeline hears them.

use crate::{Clip, Fps};

use super::Words;

/// One word, named, where it falls on the timeline the clip playing it is on.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// What a page calls it: [`Words::names`]'s, without the clip.
    pub name: String,
    /// The word as written.
    pub text: String,
    /// When it begins, in seconds of the clip's timeline.
    pub start: f64,
    /// When it ends, in the same seconds.
    pub end: f64,
}

impl Words {
    /// The words `clip` plays, on its timeline at `fps`: a word said `s`
    /// seconds into the audio is heard at `start + (s − source_in) / speed`.
    /// A word the clip's trim leaves wholly unheard is left out; one cut
    /// part-way is kept, running past the clip's edge.
    pub fn placed(&self, clip: &Clip, fps: Fps) -> Vec<Placed> {
        let speed = clip.speed.get();
        let opens = fps.seconds(clip.source_in);
        let closes = opens + fps.seconds(clip.duration) * speed;
        let starts = fps.seconds(clip.start);
        let heard = |s: f64| starts + (s - opens) / speed;
        self.words
            .iter()
            .zip(self.names())
            .filter(|(word, _)| word.end > opens && word.start < closes)
            .filter_map(|(word, name)| {
                Some(Placed {
                    name: name?,
                    text: word.text.clone(),
                    start: heard(word.start),
                    end: heard(word.end),
                })
            })
            .collect()
    }
}
