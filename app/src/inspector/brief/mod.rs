//! The brief behind a generated asset: what was asked for, and what asking that
//! way has fixed.
//!
//! The one panel here that edits an **asset** rather than a clip, because that
//! is where a brief lives. Two clips can show the same generated shot, and
//! there is one prompt between them.
//!
//! # Two briefs, and the difference is load-bearing
//!
//! A `generated_video` asset carries a sentence for a video model; a
//! `generated_audio` asset carries a line for a voice. `scorsese-core` keeps
//! those apart on purpose — different fields, different rate table, different
//! vendor — so this panel keeps them apart too, as [`shot`] and [`line`], and
//! shares only what is genuinely shared: the heading, and the one route an edit
//! takes back into the document.
//!
//! Collapsing them into one grid of *whatever fields this asset happens to
//! have* was the alternative, and it is the shape that produced the bug the
//! generate dialog's regression test is named for — a narration priced as an
//! eight-second shot, because the code had stopped being able to tell them
//! apart.
//!
//! # Constraints are shown, not discovered
//!
//! This is where most of the value is, and both halves obey it. Where a choice
//! fixes another field, the fixed field says so *instead of* accepting a value
//! and failing afterwards: `8s — locked, because 1080p` on a shot, and a
//! language that is not offered at all on the one speech model that silently
//! drops it. `scorsese check` refusing the same document an hour later is the
//! guarantee; this is the courtesy. Both exist, and the order matters — a rule
//! that fires after somebody has written a brief is a worse experience than a
//! field that was never wrong.
//!
//! **Every one of those rules is asked, never restated.** The lock comes from
//! `VideoRequest::eight_second_lock`, whether a tier takes reference images
//! from `VideoModel::supports_reference_images`, and whether a language means
//! anything from [`SpeechModel::takes_language`](scorsese_core::SpeechModel::takes_language)
//! — the same functions validation uses. A window holding its own copy of the
//! rules would be a second answer, and the first person to find them
//! disagreeing would be somebody who had already paid for a generation.

mod line;
mod shot;

use egui::Ui;
use scorsese_core::{Asset, AssetId, Project};

use super::selected::Selected;
use super::{Inspector, Refusal};
use crate::project::Open;

pub(in crate::inspector) use line::Voices;

/// The brief of whatever a clip is showing, when the asset has one.
///
/// An enum rather than a struct with two optional halves, so that a panel
/// drawing one of them cannot accidentally read a field belonging to the other.
pub(super) enum Brief {
    /// A generated shot: a prompt, a model, a raster, a length, stills.
    Shot(shot::Shot),
    /// A generated line: words, a voice, a model, and sometimes a language.
    Line(line::Line),
}

impl Brief {
    /// Reads the brief of `asset`, or `None` when it has none — which is most
    /// assets.
    pub(super) fn of(project: &Project, asset: &AssetId) -> Option<Self> {
        shot::Shot::of(project, asset)
            .map(Self::Shot)
            .or_else(|| line::Line::of(project, asset).map(Self::Line))
    }
}

impl Inspector {
    /// Draws the brief panel under the clip's own fields.
    pub(super) fn brief(
        &mut self,
        ui: &mut Ui,
        open: &mut Open,
        selected: &Selected,
        brief: &Brief,
    ) {
        ui.add_space(10.0);
        ui.label(crate::theme::marks::subheading("Brief"));
        match brief {
            Brief::Shot(shot) => self.shot(ui, open, selected, shot),
            Brief::Line(line) => self.line(ui, open, selected, line),
        }
    }

    /// Tries a change to a brief, and remembers why not when it is refused.
    ///
    /// A value the brief has already fixed is not offered in the first place, so
    /// reaching this with an impossible request takes an outside edit — which is
    /// exactly the case the refusal is for.
    pub(super) fn attempt_brief(
        &mut self,
        open: &mut Open,
        selected: &Selected,
        asset: &AssetId,
        what: impl Into<String>,
        change: impl FnOnce(&mut Asset),
    ) {
        // Keyed on the *clip*, like every other refusal here, even though the
        // change was to an asset: what dismisses a refusal is leaving the thing
        // on screen, and what is on screen is a clip.
        self.refused = super::edit::apply_to_asset(open, asset, change)
            .err()
            .map(|problems| Refusal {
                clip: selected.clip.clone(),
                what: what.into(),
                problems,
            });
    }
}
