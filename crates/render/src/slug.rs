//! Slug cards: what a clip shows when its media does not exist.
//!
//! A prompt-backed asset is a promise — a sentence, a state, and, once GO has
//! run and the money has been spent, a file. Before that file exists the clip
//! still has to render, because the point of the sketch lifecycle is that
//! watching a cut costs nothing: an agent assembles a whole video of prompts,
//! watches it, re-cuts it, and only then decides to pay. What renders is a
//! **slug card** — the prompt on a gray card, with what kind of prompt it is
//! and what state it is in written above it.
//!
//! There is no card path through the renderer. A card is a panel of colour and
//! a block of text ([`scorsese_compositor::card`]) drawn into an ordinary layer
//! buffer, which is then composited, transformed and faded by the same code a
//! decoded video frame goes through. What lives here is the two things the
//! compositor cannot know:
//!
//! - **What the card says**, which comes off the assets table.
//! - **What it looks like**, which is scorsese's own furniture rather than
//!   anything the document chooses — so every constant of it is in one `Look`,
//!   written down once. The generality rule cuts the other way here than it
//!   does for a `text` asset: a title's colour is the author's to pick because
//!   it is *in* the video, while a slug card is the editor telling you what is
//!   not there yet, and it is never in the delivered film.
//!
//! This module is also where the **disk** enters the question. The plan decides
//! media-or-card from the document alone; [`standing`] is where a `generated`
//! asset whose file has gone missing turns into a card and a note instead of a
//! failed render.

use std::path::{Path, PathBuf};

use scorsese_compositor::card::{self, Card};
use scorsese_compositor::text::{Band, Font, Style};
use scorsese_compositor::{Frame, Resolution};
use scorsese_core::{Asset, AssetKind, GenerationState, Rgba, TextAlign};

use crate::error::RenderError;
use crate::plan::{Shot, Showing};

/// The ink every card is set in. White on a dark panel, which is the pairing
/// that stays readable at preview resolutions.
const INK: Rgba = Rgba::WHITE;

/// Baseline to baseline, as a multiple of the em size.
const LINE_HEIGHT: f64 = 1.3;

/// How wide a card's text may run, as a fraction of the frame's width.
const MAX_WIDTH: f64 = 0.86;

/// What separates the label from the prompt: a line break, so the two are one
/// block that wraps and truncates together and the label can never be the part
/// that falls off the bottom.
const BREAK: &str = "\n";

/// How a card is set, as fractions of the frame it is drawn on.
///
/// Two of these exist, and which one an asset gets follows from its kind. That
/// is the whole of the styling decision, in one place.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Look {
    /// The panel's colour, alpha included.
    background: Rgba,
    /// Em size, as a fraction of the frame's height.
    size: f64,
    /// How much of the frame's height the card covers, measured up from the
    /// bottom edge. `1.0` is the whole frame.
    height: f64,
}

impl Look {
    /// A card that *is* the picture: an opaque gray panel over the whole frame,
    /// which is what a video prompt renders as when there is no video.
    const PICTURE: Self = Self {
        background: Rgba::opaque(0x3c, 0x3c, 0x3c),
        size: 0.06,
        height: 1.0,
    };

    /// A card that sits *over* the picture. A narration prompt is on an audio
    /// track: its sound is silence until it is generated, and its card has to
    /// share the frame with whatever shot it is narrating. So it is a band
    /// across the foot of the picture and it is translucent — the shot stays
    /// visible behind the words, the way a subtitle leaves the film watchable.
    const NARRATION: Self = Self {
        background: Rgba::new(0x14, 0x14, 0x14, 0xd8),
        size: 0.045,
        height: 0.3,
    };

    /// Which of the two a kind gets: sound over the picture, picture instead
    /// of it.
    fn of(kind: AssetKind) -> Self {
        if kind.is_audible() {
            Self::NARRATION
        } else {
            Self::PICTURE
        }
    }

    /// The strip of `resolution` this look occupies.
    fn band(self, resolution: Resolution) -> Band {
        let height = f64::from(resolution.height());
        Band {
            top: (height * (1.0 - self.height)) as f32,
            height: (height * self.height) as f32,
        }
    }

    /// The look in the pixels of one raster. Fractions in, pixels out — the
    /// same conversion `text` does for a title, for the same reason: a card
    /// written in pixels would be a different card at every resolution.
    fn style(self, resolution: Resolution) -> Style {
        let size = self.size * f64::from(resolution.height());
        Style {
            // A card is a stand-in drawn across the raster, not a title someone
            // laid out: it is centred whatever the clip's anchor says, because
            // the anchor is about where the *shot* sits and this is not the
            // shot.
            anchor: scorsese_core::Anchor::default(),
            size: size as f32,
            color: INK,
            align: TextAlign::Center,
            line_height: (size * LINE_HEIGHT) as f32,
            max_width: (MAX_WIDTH * f64::from(resolution.width())) as f32,
        }
    }
}

/// Why an asset has no media to show — the second half of what its card says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Absent {
    /// Nobody has spent money on this prompt yet. Where every prompt starts,
    /// and not a fault of any kind.
    Sketch,
    /// Handed to the provider and in flight; the file lands when it lands.
    Queued,
    /// Generated once, then the prompt was edited — so the file on disk is no
    /// longer what the project asks for, and showing it would be showing the
    /// wrong shot.
    Stale,
    /// The table says the media exists and it is not on disk: deleted,
    /// half-copied, or never carried along with the project. The one variant
    /// that is a problem rather than a stage.
    Gone,
}

impl Absent {
    /// What the document says about this asset's media.
    ///
    /// The document alone — no disk is consulted, which is what lets a
    /// description say why a clip is a card without a render having happened.
    /// `standing` is the same question once the filesystem has had its say.
    pub fn of(asset: &Asset) -> Self {
        match asset.state {
            Some(GenerationState::Sketch) => Self::Sketch,
            Some(GenerationState::Queued) => Self::Queued,
            Some(GenerationState::Stale) => Self::Stale,
            // `generated` with no path is a table that contradicts itself,
            // which reads exactly like a file that has gone.
            _ => Self::Gone,
        }
    }

    /// True when this is worth a note on the render report. A prompt waiting to
    /// be generated is the normal state of a project before GO; a file that
    /// should be there and is not is something somebody has to fix.
    pub const fn is_a_problem(self) -> bool {
        matches!(self, Self::Gone)
    }

    /// How the card says it.
    const fn label(self) -> &'static str {
        match self {
            Self::Sketch => "SKETCH",
            Self::Queued => "QUEUED",
            Self::Stale => "STALE",
            Self::Gone => "MEDIA MISSING",
        }
    }
}

impl std::fmt::Display for Absent {
    /// In the document's own words, lowercase: these are the `state` values a
    /// reader would go and look for in `project.json`. The card's own label
    /// shouts, which is right on a gray card and wrong in the middle of a
    /// sentence.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Sketch => "sketch",
            Self::Queued => "queued",
            Self::Stale => "stale",
            Self::Gone => "media missing",
        })
    }
}

/// What a shot shows once the disk has been consulted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Standing {
    /// The asset's own media, resolved against the project root and known to
    /// be there.
    Media(PathBuf),
    /// A card, because there is nothing to decode.
    Card(Absent),
}

/// Whether this shot's media is there, and what stands in for it if not.
///
/// The one place a render asks the filesystem what the document believes. A
/// missing **generated** file becomes a card rather than a refusal: the media
/// can be made again, the preview is still worth watching, and a render that
/// died on it would be a render nobody could use to find out what else is
/// wrong. A missing **imported** file stays a refusal, because nothing but the
/// author can put it back and silently standing in for it would hide the loss
/// of footage inside a finished-looking file.
pub(crate) fn standing(shot: &Shot<'_>, project_root: &Path) -> Result<Standing, RenderError> {
    if shot.showing == Showing::Card {
        return Ok(Standing::Card(Absent::of(shot.asset)));
    }
    let path = shot
        .asset
        .path
        .as_ref()
        .expect("the plan gives a shot showing media a path");
    let file = path.resolve(project_root);
    if file.is_file() {
        return Ok(Standing::Media(file));
    }
    if shot.asset.kind.is_generated() {
        return Ok(Standing::Card(Absent::Gone));
    }
    Err(RenderError::MissingMedia {
        asset: shot.asset.id.to_string(),
        path: file,
    })
}

/// Draws `asset`'s card into `frame`, which is cleared first.
///
/// Everywhere the card is not — the whole frame but for a narration band — the
/// layer stays transparent, so the tracks below it show through.
pub(crate) fn paint(frame: &mut Frame, asset: &Asset, absent: Absent) {
    let look = Look::of(asset.kind);
    let resolution = frame.resolution();
    frame.fill_transparent();
    card::draw(
        frame,
        &Card {
            band: look.band(resolution),
            background: look.background,
            text: &wording(asset, absent),
            style: look.style(resolution),
        },
        Font::sans(),
    );
}

/// What the card says, in the order it is read: what this is, then what it was
/// going to be.
///
/// Public because it is the sentence a person reads off a slug card, and
/// anything that wants to say the same thing without painting a frame — a
/// tooltip, a description, a test — should say it the same way rather than
/// building its own.
///
/// The label is a line of the same block rather than a badge drawn separately,
/// which is what makes a card one wrap and one truncation: a prompt longer than
/// the frame loses its tail to an ellipsis and never its label.
pub fn wording(asset: &Asset, absent: Absent) -> String {
    let (kind, brief) = brief(asset);
    format!("{kind} · {}{BREAK}{brief}", absent.label())
}

/// What kind of brief this asset carries, and what that brief says.
///
/// The three generated kinds do not all carry a prompt. A `synth_audio` asset
/// is made from a **recipe** in the project, so asking it for a prompt and
/// finding none used to put "(no prompt)" on the card — reporting a perfectly
/// well-formed asset as broken, in the one place a person looks to find out
/// what a shot is going to be. The card names what is actually there.
fn brief(asset: &Asset) -> (&'static str, &str) {
    if asset.kind.is_synthesized() {
        let recipe = asset
            .recipe
            .as_ref()
            .map_or("(no recipe)", |recipe| recipe.as_str());
        return ("AUDIO RECIPE", recipe);
    }
    let kind = if asset.kind.is_audible() {
        "AUDIO PROMPT"
    } else {
        "VIDEO PROMPT"
    };
    (kind, asset.prompt.as_deref().unwrap_or("(no prompt)"))
}
