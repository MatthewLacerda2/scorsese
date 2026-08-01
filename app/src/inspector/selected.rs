//! What the panel knows about the selected clip, read out of the document in
//! one pass.
//!
//! A snapshot rather than a borrow, and that is what makes an editing panel
//! possible at all: a control that changes the project cannot also be holding a
//! reference into it. Reading everything first and drawing from the copy keeps
//! the borrow short and the drawing code free of the question.

use scorsese_core::{
    AssetId, AssetKind, ClipId, Fit, Frames, GenerationState, KeyframeTrack, Project, PropertyPath,
    Speed, TrackId, TrackKind,
};

/// One clip, as the inspector shows it.
pub(super) struct Selected {
    /// Which clip this is — the handle every edit goes back through.
    pub(super) clip: ClipId,
    /// The asset it shows. Its id, because that is what the document, the CLI
    /// and an assistant all call it.
    pub(super) asset: AssetId,
    /// What that asset is, when the id resolves to one. A loaded project is
    /// validated, so `None` is not a state a person can normally reach — it
    /// still gets said rather than guessed at.
    pub(super) kind: Option<AssetKind>,
    /// Where a generated asset is in the sketch lifecycle, which is the answer
    /// to "why is this a slug card?" for anyone who has not asked yet.
    pub(super) state: Option<GenerationState>,
    /// The track the clip sits on.
    pub(super) track: TrackId,
    /// True when that track carries picture. `fit` is a question about a
    /// raster, and a sound has none.
    pub(super) picture: bool,
    /// Where the clip begins, in frames.
    pub(super) start: Frames,
    /// How long it runs, in frames.
    pub(super) duration: Frames,
    /// How the source meets the render's raster.
    pub(super) fit: Fit,
    /// How fast it runs against the timeline.
    pub(super) speed: Speed,
    /// Every property something animates over this clip.
    pub(super) animated: Vec<Animated>,
}

/// One property a clip animates, as much of it as a panel that does not edit
/// keyframes has any business knowing.
pub(super) struct Animated {
    /// The property path as the document writes it — `opacity`, `volume`,
    /// `transform.position.x`. Shown verbatim, because the app deliberately
    /// holds no list of which properties exist: that list lives in the crates
    /// that implement them, and a window that kept its own copy would go out
    /// of date the first time one was added.
    pub(super) property: PropertyPath,
    /// How many points the ramp has. A count, not the points: the shape of an
    /// animation is the timeline's picture to draw, and this panel's job is
    /// only to say that there is one.
    pub(super) points: usize,
    /// The tool that wrote this track, if a tool did. Absent means a person or
    /// an assistant placed the points by hand — the difference that decides
    /// whether deleting the whole track is offered.
    pub(super) by: Option<String>,
}

impl Selected {
    /// Reads a clip out of the project, or `None` when the id names none —
    /// which happens when the selection outlives the clip it pointed at.
    pub(super) fn of(project: &Project, clip: &ClipId) -> Option<Self> {
        let (track, found) = project.clips().find(|(_, found)| &found.id == clip)?;
        let asset = project.asset(&found.asset);
        Some(Self {
            clip: found.id.clone(),
            asset: found.asset.clone(),
            kind: asset.map(|asset| asset.kind),
            state: asset.and_then(|asset| asset.state),
            track: track.id.clone(),
            picture: track.kind == TrackKind::Video,
            start: found.start,
            duration: found.duration,
            fit: found.fit,
            speed: found.speed,
            animated: found.keyframes.iter().map(Animated::of).collect(),
        })
    }
}

impl Animated {
    /// One keyframe track, summarised.
    fn of(track: &KeyframeTrack) -> Self {
        Self {
            property: track.property.clone(),
            points: track.keyframes.len(),
            by: track.by.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector::fixture::project;
    use scorsese_core::{Easing, Keyframe};

    /// A keyframe at `t` — the value and the easing are beside the point here.
    fn at(t: u64) -> Keyframe {
        Keyframe {
            t: Frames(t),
            value: 1.0,
            easing: Easing::Linear,
        }
    }

    /// The panel's whole claim about animation: which properties have a ramp
    /// on them, how many points it has, and which of them a tool signed.
    #[test]
    fn every_animated_property_is_reported_with_who_wrote_it() {
        let mut project = project();
        let clip = &mut project.tracks[0].clips[0];
        clip.keyframes = vec![
            KeyframeTrack::new(PropertyPath::new("opacity"), vec![at(0), at(12)]),
            KeyframeTrack::new(PropertyPath::new("volume"), vec![at(0)]).generated_by("duck"),
        ];

        let selected = Selected::of(&project, &ClipId::new("c1")).expect("the clip is there");
        let animated: Vec<(&str, usize, Option<&str>)> = selected
            .animated
            .iter()
            .map(|animated| {
                (
                    animated.property.as_str(),
                    animated.points,
                    animated.by.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            animated,
            [("opacity", 2, None), ("volume", 1, Some("duck"))],
            "a hand-written ramp and a tool's must not read the same"
        );
    }

    /// A clip animating nothing says so by having nothing to say — not by an
    /// empty heading a person has to read to learn there is no news.
    #[test]
    fn a_clip_with_no_ramps_reports_none() {
        let project = project();
        let selected = Selected::of(&project, &ClipId::new("c1")).expect("the clip is there");
        assert!(selected.animated.is_empty());
        assert_eq!(selected.start, Frames(0));
        assert!(selected.picture, "v1 is a video track");
    }

    /// `fit` is a question about a raster and a sound has none, so the panel
    /// has to be able to tell which of the two it is looking at.
    #[test]
    fn a_clip_on_an_audio_track_is_not_picture() {
        let selected = Selected::of(&project(), &ClipId::new("c3")).expect("the clip is there");
        assert!(!selected.picture);
        assert_eq!(selected.kind, Some(AssetKind::Audio));
    }

    /// The selection is a session fact and the document is not: a clip that has
    /// gone leaves the panel with nothing to show, never with a stale copy.
    #[test]
    fn a_clip_that_is_not_in_the_document_selects_nothing() {
        assert!(Selected::of(&project(), &ClipId::new("gone")).is_none());
    }
}
