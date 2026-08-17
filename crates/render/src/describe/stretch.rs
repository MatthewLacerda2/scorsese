//! One stretch of timeline, and what is on it.

use scorsese_compositor::ANIMATED as DRAWN;
use scorsese_core::{AssetKind, Crop, Fit, Fps, Icon, Rgba, Shape, Speed};

use crate::audio::gain::ANIMATED as MIXED;
use crate::audio::path::VOLUME;
use crate::plan::{Segment, Shot, Showing};
use crate::slug::Absent;

use super::Moment;
use super::animation::{self, Animated};

/// A stretch of timeline over which neither the picture nor the mix changes.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    /// Where it begins on the timeline.
    pub from: Moment,
    /// Where it ends — the frame the next stretch owns, so `from..to` is
    /// half-open and consecutive stretches never claim the same frame.
    pub to: Moment,
    /// The frame of the **delivered file** this stretch begins on, which is
    /// what a still is pulled from.
    pub at_frame: u64,
    /// What is on screen throughout, **bottom track first**. Empty is a gap:
    /// nothing on any video track, which renders black.
    pub picture: Vec<Playing>,
    /// What is in the mix throughout, in track order. Empty is silence.
    pub sound: Vec<Playing>,
}

impl Stretch {
    /// True when nothing at all is on screen here — two seconds of black, not
    /// two seconds of missing time.
    pub fn is_gap(&self) -> bool {
        self.picture.is_empty()
    }

    /// Reads one stretch off the segments covering it.
    pub(super) fn between(
        from: Moment,
        to: Moment,
        at_frame: u64,
        picture: Option<&Segment<'_>>,
        sound: Option<&Segment<'_>>,
        fps: Fps,
    ) -> Self {
        let read = |segment: Option<&Segment<'_>>, vocabulary| {
            segment.map_or_else(Vec::new, |segment| {
                segment
                    .layers
                    .iter()
                    .map(|shot| Playing::of(shot, vocabulary, from, to, fps))
                    .collect()
            })
        };
        Self {
            from,
            to,
            at_frame,
            picture: read(picture, DRAWN),
            sound: read(sound, MIXED),
        }
    }
}

/// One clip present through a stretch, on the picture or in the mix.
///
/// The same clip can be both, and legitimately appears twice: a shot with
/// dialogue on it is a layer and a sound, and an ungenerated narration prompt
/// is a card on the picture *and* silence in the mix. Saying it once would mean
/// leaving out one of the two things it does.
#[derive(Debug, Clone, PartialEq)]
pub struct Playing {
    /// The track it sits on — which lane, and for picture, which layer.
    pub track: String,
    /// The clip, by the id every render note and validation error names it by.
    pub clip: String,
    /// The asset it shows, by id.
    pub asset: String,
    /// What kind of media that asset is.
    pub kind: AssetKind,
    /// Media, inline text, or a card standing in for something ungenerated.
    pub shows: Shown,
    /// How the source meets the raster. **Picture only** — an audio clip has no
    /// raster and this says nothing about one.
    pub fit: Fit,
    /// Which part of the source is shown, when it is not all of it.
    ///
    /// Said out loud rather than left to whoever opens the document, because a
    /// crop is the one clip property whose effect a reader would otherwise
    /// attribute to the asset — the shot simply *looks* like that — and the
    /// whole point of the field is that the asset is untouched.
    pub crop: Option<Crop>,
    /// How fast the clip runs against the timeline.
    ///
    /// Said for the same reason a crop is: it is a property of the *clip* whose
    /// effect a reader would otherwise attribute to the asset — the shot simply
    /// *moves* like that — and it is the one thing about a stretch that a frame
    /// count cannot reveal. A ten-second slot holding twenty seconds of footage
    /// looks exactly like a ten-second slot until something says so.
    ///
    /// Applies to picture and sound alike, which is why it is on both lines.
    pub speed: Speed,
    /// The properties animated across this stretch, in the order the document
    /// writes them.
    ///
    /// Only properties something in this build actually animates: a keyframe
    /// track naming a typo is reported by [`crate::properties::unknown_in`] and
    /// left out here, because describing a fade that will never happen is worse
    /// than not describing it.
    pub animated: Vec<Animated>,
}

impl Playing {
    /// True when this entry — read from [`Stretch::sound`] — actually puts
    /// something into the mix.
    ///
    /// False for a card, whose sound has not been generated and is silence, and
    /// for a clip held at zero volume throughout: both are clips a reader would
    /// otherwise take for audible, and both are exactly the mistake this
    /// description exists to surface.
    pub fn is_audible(&self) -> bool {
        if matches!(self.shows, Shown::Card { .. }) {
            return false;
        }
        !self
            .animated
            .iter()
            .any(|animated| animated.property == VOLUME && animated.is_zero())
    }

    fn of(
        shot: &Shot<'_>,
        vocabulary: &'static [scorsese_compositor::Property],
        from: Moment,
        to: Moment,
        fps: Fps,
    ) -> Self {
        Self {
            track: shot.track.to_string(),
            clip: shot.clip.id.to_string(),
            asset: shot.asset.id.to_string(),
            kind: shot.asset.kind,
            shows: Shown::of(shot),
            fit: shot.clip.fit,
            crop: shot.clip.crop,
            speed: shot.clip.speed,
            animated: animation::across(shot.clip, vocabulary, from.frame, to.frame, fps),
        }
    }
}

/// What a clip puts there — the things a layer can actually be.
///
/// `PartialEq` and no `Eq`, since a shape carries the fractions it is drawn
/// from and a fraction is a float. Nothing sorts or hashes a description, so
/// the reflexivity `Eq` promises buys nothing here that would pay for keeping
/// the geometry out.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    /// The asset's own media, decoded from the file it names.
    Media,
    /// Content carried inline in the document and drawn by the compositor.
    Text(String),
    /// A solid colour filling the raster. The other inline kind, and the one
    /// a description has to name rather than call "media": a reader counting
    /// layers should be able to tell a background from a shot.
    Color(Rgba),
    /// A drawn box or ellipse. Named for [`Shown::Color`]'s reason and one of
    /// its own: a diagram is a dozen small layers, and a reader working out
    /// which of them came out wrong needs each described as the shape it is
    /// rather than counted as media.
    Shape(Shape),
    /// A named symbol from the set this build ships. Named for
    /// [`Shown::Shape`]'s reason and one more: the name is the one part of an
    /// icon that can be wrong without anything else about the clip looking odd,
    /// so a description that hid it behind "media" would hide the mistake too.
    Icon(Icon),
    /// A slug card, because there is nothing generated to show. What makes a
    /// full preview cut of prompt clips cost nothing — and the case a
    /// description most has to name, since a cut made entirely of cards looks
    /// like a finished edit to anything counting frames.
    Card {
        /// Why there is no media: where the prompt sits in the lifecycle, or
        /// that its file has gone.
        absent: Absent,
        /// The prompt written on the card.
        prompt: String,
    },
}

impl Shown {
    fn of(shot: &Shot<'_>) -> Self {
        if shot.showing == Showing::Card {
            return Self::Card {
                absent: Absent::of(shot.asset),
                prompt: shot
                    .asset
                    .prompt
                    .clone()
                    .unwrap_or_else(|| "(no prompt)".to_owned()),
            };
        }
        match (shot.asset.kind, shot.asset.text.as_ref()) {
            (AssetKind::Text, Some(text)) => Self::Text(text.clone()),
            (AssetKind::Color, _) => Self::Color(shot.asset.color.unwrap_or_default()),
            (AssetKind::Shape, _) => match &shot.asset.shape {
                Some(shape) => Self::Shape(shape.clone()),
                // Validation requires it, so this is only reachable through an
                // asset built in memory. Media rather than an invented
                // rectangle: describing a shape nobody wrote would be worse
                // than describing nothing.
                None => Self::Media,
            },
            // Same reading, and the same unreachable arm: an icon nobody named
            // is better described as nothing than as a symbol invented here.
            (AssetKind::Icon, _) => match &shot.asset.icon {
                Some(icon) => Self::Icon(icon.clone()),
                None => Self::Media,
            },
            _ => Self::Media,
        }
    }
}
