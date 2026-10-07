//! Where the timeline's clips are, told to a page in its own seconds (#810).
//!
//! A page that runs beside a whole edit times itself against things that are
//! not in it — the narration line it lights a block for, the beat of the music
//! it lands a title on. Copying those times into the page by hand is the edit
//! written down twice, and the copy goes stale silently the first time a clip
//! moves. So a page is told `scorsese.clips`: every clip on the timeline its own
//! clip sits on, by id, as `{start, end}` in **page seconds** — the clock
//! `scorsese.duration` is on, already corrected for the page clip's own
//! placement, `source_in` and `speed`. A clip before the page's clip starts
//! begins at a negative time; one after it ends begins past `duration`.
//!
//! **Only what a page read is part of its capture.** `scorsese.clips` notes
//! every name the page asks it for — present or not — and whether the page
//! listed them all (`Object.keys`, a `for … in`, `JSON.stringify`). A capture
//! keeps the place of each clip it read ([`Told`]), and is fresh while those
//! places are unchanged: moving the narration re-captures the page that times
//! itself to it, and no other.

use std::collections::{BTreeMap, BTreeSet};

use scorsese_core::hash_bytes;
use serde::{Deserialize, Serialize};

use super::request::Request;

/// How far apart two places of a clip may be and still be the same: far less
/// than a frame at any rate, far more than a rounding.
const SAME: f64 = 1e-6;

/// Where one clip sits, in seconds of the page's clock.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Span {
    /// Where the clip begins.
    pub start: f64,
    /// Where it ends.
    pub end: f64,
}

/// What a page asked `scorsese.clips` for, as the browser reports it after the
/// capture's last frame.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct Read {
    /// Every name it looked up, whether or not a clip has it.
    pub(crate) names: BTreeSet<String>,
    /// Whether it listed every clip.
    pub(crate) listed: bool,
}

impl Read {
    /// What two pieces of one capture read, as one capture would have.
    pub(crate) fn merge(&mut self, other: Self) {
        self.names.extend(other.names);
        self.listed |= other.listed;
    }
}

/// What a capture's frames depend on of the clips: the place it was told of
/// each one it read.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Told {
    /// Each name read, with the clip's place — `None` for a name no clip had,
    /// which a clip given that id later makes stale. Every clip, when listed.
    pub(crate) clips: BTreeMap<String, Option<Span>>,
    /// Whether the page listed the clips, which makes which clips there are
    /// part of what it drew.
    pub(crate) listed: bool,
}

impl Told {
    /// What `request` told a page that read `read`.
    pub(crate) fn of(request: &Request, read: &Read) -> Self {
        let listed = read.listed.then(|| request.clips.keys().cloned());
        let names = read
            .names
            .iter()
            .cloned()
            .chain(listed.into_iter().flatten());
        Self {
            clips: names
                .map(|name| {
                    let span = request.clips.get(&name).copied();
                    (name, span)
                })
                .collect(),
            listed: read.listed,
        }
    }

    /// Whether `request` would tell a page reading the same names the same
    /// thing — so a capture made with this is still one of `request`.
    ///
    /// Places are compared to within [`SAME`], not to the bit: a record read
    /// back from JSON can be a rounding away from what was written, and a clip
    /// that really moved has moved by a frame at least.
    pub(crate) fn holds_for(&self, request: &Request) -> bool {
        let read = Read {
            names: self.clips.keys().cloned().collect(),
            listed: self.listed,
        };
        let now = Self::of(request, &read);
        let same = |a: &Option<Span>, b: &Option<Span>| match (a, b) {
            (Some(a), Some(b)) => (a.start - b.start).abs() < SAME && (a.end - b.end).abs() < SAME,
            (None, None) => true,
            _ => false,
        };
        now.clips.len() == self.clips.len()
            && now
                .clips
                .iter()
                .zip(&self.clips)
                .all(|((name, span), (was, then))| name == was && same(span, then))
    }

    /// A name for the folder its captures are kept in: the same for every
    /// capture told the same thing, and `told-none` for a page that read
    /// nothing, which is most of them.
    pub(crate) fn folder(&self) -> String {
        if *self == Self::default() {
            return "told-none".to_owned();
        }
        let json = serde_json::to_string(self).expect("a told always serialises");
        format!("told-{}", &hash_bytes(json.as_bytes())[..16])
    }
}

/// The script that hands a page `contract` and `clips` as `window.scorsese`,
/// and keeps what it reads of the clips for `__scorseseTold()` to say.
pub(crate) const SCRIPT: &str = include_str!("told.js");

/// The expression the capture asks the page, after its last frame, for what it
/// read.
pub(crate) const ASK: &str = "__scorseseTold()";

#[cfg(test)]
mod tests {
    use scorsese_compositor::Resolution;
    use scorsese_core::Fps;

    use super::*;

    fn request(clips: &[(&str, f64)]) -> Request {
        Request {
            page: "pages/a.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps: Fps::THIRTY,
            duration: 2.0,
            clips: clips
                .iter()
                .map(|(id, start)| {
                    let span = Span {
                        start: *start,
                        end: start + 1.0,
                    };
                    ((*id).to_owned(), span)
                })
                .collect(),
        }
    }

    fn read(names: &[&str], listed: bool) -> Read {
        Read {
            names: names.iter().map(|n| (*n).to_owned()).collect(),
            listed,
        }
    }

    #[test]
    fn a_capture_holds_while_the_clips_it_read_stay_put() {
        let before = request(&[("vo", 1.0), ("music", 0.0)]);
        let told = Told::of(&before, &read(&["vo", "ghost"], false));
        assert_eq!(told.clips["vo"].unwrap().start, 1.0);
        assert_eq!(told.clips["ghost"], None);
        assert!(told.holds_for(&before));
        let json = serde_json::to_string(&told).unwrap();
        let back: Told = serde_json::from_str(&json).unwrap();
        assert!(back.holds_for(&before), "read back from its record");
        assert!(told.holds_for(&request(&[("vo", 1.0 + 1e-12), ("music", 0.0)])));
        assert!(
            told.holds_for(&request(&[("vo", 1.0), ("music", 5.0)])),
            "a clip it never read moved"
        );
        assert!(!told.holds_for(&request(&[("vo", 1.5), ("music", 0.0)])));
        assert!(
            !told.holds_for(&request(&[("vo", 1.0), ("ghost", 0.0)])),
            "a name it missed now has a clip"
        );
    }

    #[test]
    fn a_page_that_listed_the_clips_depends_on_every_one() {
        let before = request(&[("vo", 1.0)]);
        let told = Told::of(&before, &read(&[], true));
        assert!(told.holds_for(&before));
        assert!(!told.holds_for(&request(&[("vo", 1.0), ("new", 3.0)])));
        assert!(!told.holds_for(&request(&[])));
    }

    #[test]
    fn captures_told_alike_share_a_folder() {
        let one = request(&[("vo", 1.0)]);
        let nothing = Told::of(&one, &Read::default());
        assert_eq!(nothing.folder(), "told-none");
        let vo = Told::of(&one, &read(&["vo"], false));
        assert_eq!(vo.folder(), Told::of(&one, &read(&["vo"], false)).folder());
        assert_ne!(vo.folder(), nothing.folder());
        let moved = Told::of(&request(&[("vo", 2.0)]), &read(&["vo"], false));
        assert_ne!(vo.folder(), moved.folder());
    }

    #[test]
    fn pieces_read_what_any_of_them_read() {
        let mut first = read(&["a"], false);
        first.merge(read(&["b"], true));
        assert_eq!(first, read(&["a", "b"], true));
    }
}
