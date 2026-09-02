//! Every colour the window uses, named once.
//!
//! A panel that reaches for a literal `Color32` is a panel whose look cannot be
//! changed without finding it, and three of them had drifted apart before this
//! module existed — a refusal red in the inspector, an attention orange in the
//! pool, and a clip fill in the timeline, all invented separately and none of
//! them the same as any other.
//!
//! ## The scheme
//!
//! Near-black grounds, hairline rules, one cool accent and one warm one. The
//! grounds are cool rather than neutral — a few points of blue in a very dark
//! grey is what keeps a large dark surface from reading as *off* — and they
//! step in small increments, because the hierarchy is carried by the rules and
//! the accents rather than by tonal contrast between panels.
//!
//! Colour means one thing here and only one: **what a thing is**. It is never
//! decoration and never a mood. That is why the clip hues live in this file
//! beside the chrome — a title is gold wherever it is drawn, and the pool's
//! chip and the timeline's block have to agree or the colour is telling two
//! stories.

use egui::Color32;
use scorsese_core::{AssetKind, TrackKind};

/// Behind everything: the window's own ground, and the matte around a picture.
pub(crate) const VOID: Color32 = Color32::from_rgb(0x08, 0x0A, 0x0D);
/// A panel's fill.
pub(crate) const INK: Color32 = Color32::from_rgb(0x0F, 0x12, 0x17);
/// A surface standing on a panel: a lane, a field, a button at rest.
pub(crate) const RAISED: Color32 = Color32::from_rgb(0x17, 0x1C, 0x23);
/// The same, under the pointer.
pub(crate) const HOVER: Color32 = Color32::from_rgb(0x21, 0x28, 0x32);
/// And while it is being pressed or dragged.
pub(crate) const ACTIVE: Color32 = Color32::from_rgb(0x2C, 0x36, 0x43);

/// A hairline: the separator between two things that are the same kind of
/// thing. Deliberately barely there — a rule that shouts is a rule you read
/// instead of the content beside it.
pub(crate) const RULE: Color32 = Color32::from_rgb(0x22, 0x2A, 0x34);
/// A hairline that is carrying more weight: a panel edge, the underline of a
/// section, the foot of the ruler.
pub(crate) const EDGE: Color32 = Color32::from_rgb(0x30, 0x3C, 0x49);

/// What ordinary text is set in.
pub(crate) const TEXT: Color32 = Color32::from_rgb(0xC6, 0xD1, 0xDD);
/// Text that is true but secondary: a unit, a count, a hint.
pub(crate) const DIM: Color32 = Color32::from_rgb(0x7B, 0x88, 0x96);
/// Text at the edge of legibility, for things that are there to be found
/// rather than read: a tick label, a placeholder.
pub(crate) const FAINT: Color32 = Color32::from_rgb(0x51, 0x5D, 0x69);

/// The cool accent: selection, section headings, the marks that frame a
/// picture. One accent, used sparingly, is what keeps it meaning "look here".
pub(crate) const ACCENT: Color32 = Color32::from_rgb(0x4C, 0xD2, 0xE6);
/// The same accent with the volume down, for a rule or a chip that should read
/// as *related to* the accent rather than as an alarm.
pub(crate) const ACCENT_DIM: Color32 = Color32::from_rgb(0x2A, 0x74, 0x84);

/// Something that wants attention but is not wrong: a file that changed under
/// the window, an asset nobody has made yet, a snap the drag came to rest on.
pub(crate) const WARM: Color32 = Color32::from_rgb(0xE3, 0xA1, 0x3E);
/// Something that *is* wrong: a project that will not validate, a refused
/// edit, an asset whose file has gone.
pub(crate) const ALERT: Color32 = Color32::from_rgb(0xE8, 0x5C, 0x5C);
/// The playhead, and nothing else. Its own colour on purpose: the one mark on
/// screen that must never be mistaken for a clip, a rule or a warning.
pub(crate) const PLAYHEAD: Color32 = Color32::from_rgb(0xFF, 0x4E, 0x63);

/// The hue that says what an asset is.
///
/// Six families rather than ten, because the reader is looking for a *kind of
/// thing* — footage, a still, a title, a graphic, music, a voice — and ten hues
/// on one timeline stop being a code and start being a fruit bowl. A generated
/// shot is the same blue as a shot, because what it will be is a shot; the
/// difference between made and unmade is carried by the clip's *treatment*, not
/// by its colour. See [`super::marks::hatch`].
pub(crate) const fn of_kind(kind: AssetKind) -> Color32 {
    match kind {
        // Footage, made or not.
        AssetKind::Video | AssetKind::GeneratedVideo => Color32::from_rgb(0x3E, 0x7B, 0xC8),
        // A still.
        AssetKind::Image => Color32::from_rgb(0x25, 0x9E, 0x8C),
        // Words on screen.
        AssetKind::Text => Color32::from_rgb(0xC9, 0x9B, 0x33),
        // Drawn rather than shot: a colour card, a shape, an icon.
        AssetKind::Color | AssetKind::Shape | AssetKind::Icon => {
            Color32::from_rgb(0x7E, 0x5C, 0xC4)
        }
        // Sound somebody brought or synthesised.
        AssetKind::Audio | AssetKind::SynthAudio => Color32::from_rgb(0x35, 0x9B, 0x63),
        // A voice, kept apart from music: which of the two a lane is carrying
        // is the commonest thing anybody needs to see at a glance while
        // balancing a mix.
        AssetKind::GeneratedAudio => Color32::from_rgb(0x2E, 0x93, 0xA8),
    }
}

/// The hue that says what a *lane* carries.
///
/// The same two families the clip hues use for their commonest members, so a
/// track head and the blocks along it agree. It is drawn as a bar down the edge
/// of the gutter, which is the one mark that answers "am I about to drag this
/// onto picture or onto sound?" without reading anything.
pub(crate) const fn of_track(kind: TrackKind) -> Color32 {
    match kind {
        TrackKind::Video => of_kind(AssetKind::Video),
        TrackKind::Audio => of_kind(AssetKind::Audio),
    }
}

/// The colour of a clip whose asset is not in the table at all.
///
/// A loaded project is validated, so this is not a state a person normally
/// reaches — but "drawn in the alert colour" is a great deal more use than
/// "drawn as whatever the last branch happened to leave in the variable".
pub(crate) const UNKNOWN: Color32 = ALERT;

/// A hue mixed down towards a ground, which is how a clip body is made.
///
/// `amount` is how much of the hue survives: 1.0 is the hue itself, 0.0 the
/// ground. Blended here rather than by reaching for `gamma_multiply` because
/// that darkens *towards black*, and a dark blue on a near-black panel wants to
/// keep its blue — multiplying it takes the colour out along with the light.
pub(crate) fn over(hue: Color32, ground: Color32, amount: f32) -> Color32 {
    let mix = |a: u8, b: u8| (f32::from(b) + (f32::from(a) - f32::from(b)) * amount) as u8;
    Color32::from_rgb(
        mix(hue.r(), ground.r()),
        mix(hue.g(), ground.g()),
        mix(hue.b(), ground.b()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ends of the mix are the two colours themselves. A blend that drifted
    /// at either end would make a clip body disagree with the header strip
    /// drawn from the same hue.
    #[test]
    fn a_mix_of_all_or_nothing_is_one_colour_or_the_other() {
        assert_eq!(over(ACCENT, INK, 1.0), ACCENT);
        assert_eq!(over(ACCENT, INK, 0.0), INK);
    }

    /// Every kind has a hue and none of them is the ground: a clip drawn in the
    /// panel's own colour is a clip nobody can see.
    #[test]
    fn every_kind_is_visible_against_a_lane() {
        for kind in [
            AssetKind::Video,
            AssetKind::Image,
            AssetKind::Audio,
            AssetKind::Text,
            AssetKind::Color,
            AssetKind::Shape,
            AssetKind::Icon,
            AssetKind::GeneratedVideo,
            AssetKind::GeneratedAudio,
            AssetKind::SynthAudio,
        ] {
            assert_ne!(of_kind(kind), RAISED, "{kind:?} is invisible on a lane");
        }
    }

    /// Picture and sound have to be told apart at a glance, and so do music and
    /// a voice — those are the two distinctions somebody makes most often.
    #[test]
    fn the_families_that_have_to_differ_do() {
        assert_ne!(of_kind(AssetKind::Video), of_kind(AssetKind::Audio));
        assert_ne!(
            of_kind(AssetKind::Audio),
            of_kind(AssetKind::GeneratedAudio)
        );
        assert_ne!(of_kind(AssetKind::Text), of_kind(AssetKind::Image));
        assert_eq!(
            of_kind(AssetKind::Video),
            of_kind(AssetKind::GeneratedVideo),
            "an unmade shot is still a shot"
        );
    }
}
