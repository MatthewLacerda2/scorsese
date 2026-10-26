//! The project files panel: what the edit is made of.
//!
//! The same answer `scorsese assets` gives, through the same
//! [`asset_status`] — so the window and the command line can never disagree
//! about what is in the pool or what state it is in.
//!
//! Laid out as the web editor's Assets sidebar is (#766, #1006): a grid of
//! tiles, two across, each a picture with its name under it and the colour its
//! clips have on the timeline as its edge ([`tile`]). Footage and stills show a
//! frame, made in the background ([`thumbs`]); everything else is drawn
//! ([`glyph`]). A tile says nothing of its kind or its use count — the hover
//! does — and tags its state only when it is not a made, healthy asset, because
//! that is money waiting to be spent.
//!
//! Grouped by what a thing *is*, because that is how someone looks for one:
//! footage, sound, titles. A generated asset sits with what it is, whether or
//! not it has been made yet (#1201).
//!
//! It is also where a clip comes from: a tile dragged onto a lane of the
//! timeline places a clip of it there. Right-clicked, it asks to remove the
//! asset and the clips using it ([`crate::removing`]).
//!
//! An image sequence is one tile — its first still — with its stills folded
//! behind an arrow on it (#684): a 400-photo timelapse would otherwise be 401
//! tiles, 400 of which nobody drags anywhere on their own. Unfolded, the
//! stills follow it as tiles of their own, there to be *seen* — which file,
//! whether it is healthy, whether a clip also shows it alone — and reordering
//! them stays a sentence to the assistant.

mod glyph;
pub(crate) mod thumbs;
mod tile;

use std::collections::HashSet;

use egui::{RichText, ScrollArea, Ui};
use scorsese_core::{
    AssetHealth, AssetId, AssetKind, AssetStatus, HashCheck, Listed, asset_status, listing,
};

use crate::editing::Editing;
use crate::project::Open;
use crate::removing::{Asking, Removal};
use crate::theme::{marks, palette};
use thumbs::Thumbs;
use tile::Face;

/// How many tiles sit across the side column, as the web's sidebar's two.
const ACROSS: usize = 2;
/// The gap between tiles.
const GAP: f32 = 6.0;

/// The panel's own state: the pool as it was last looked at.
#[derive(Default)]
pub(crate) struct Files {
    status: Vec<AssetStatus>,
    /// The sequences someone has unfolded. Folded is the default, so a
    /// project opens with every sequence as one tile.
    unfolded: HashSet<AssetId>,
    /// The pictures on the tiles.
    thumbs: Thumbs,
}

/// One tile to draw: an asset and its stills, or one of those stills.
enum Entry<'a> {
    Asset(Listed<'a>),
    Still(&'a AssetStatus),
}

impl Files {
    /// Re-reads the pool from disk.
    ///
    /// Cached rather than recomputed every frame, and that is not a
    /// micro-optimisation: [`asset_status`] asks the filesystem whether each
    /// file is there, and doing that sixty times a second for every asset
    /// would make scrubbing stutter for an answer that changes when someone
    /// deletes a file.
    pub(crate) fn refresh(&mut self, open: &Open) {
        // `Skip`, so this never hashes: existence is cheap and the whole pool
        // re-hashed is not. `scorsese check --verify` is where that lives.
        self.status = asset_status(&open.project, &open.root, HashCheck::Skip);
    }

    /// Forgets everything, for when a different project is opened.
    pub(crate) fn reset(&mut self) {
        self.status.clear();
        self.unfolded.clear();
        self.thumbs.reset();
    }

    /// Draws the panel.
    pub(crate) fn show(&mut self, ui: &mut Ui, open: &Open, editing: &mut Editing) {
        ui.horizontal(|ui| {
            ui.label(marks::heading("Project files"));
            if ui
                .small_button("↻")
                .on_hover_text("Re-read the pool")
                .clicked()
            {
                self.refresh(open);
            }
        });

        if self.status.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("nothing imported yet")
                    .italics()
                    .small()
                    .color(palette::of(ui.ctx()).ring),
            );
            return;
        }

        self.thumbs.poll(ui.ctx());
        // No floor under its height. egui's default makes a scroll area at
        // least 64 points tall whatever room is left, and in a short window
        // that floor pushed the side column past the bottom of the window
        // (#681): a list that shrinks to a sliver is still reachable by
        // scrolling, and a column drawn off-screen is not.
        ScrollArea::vertical()
            .min_scrolled_height(0.0)
            .show(ui, |ui| {
                for (heading, kinds) in GROUPS {
                    self.group(ui, open, editing, heading, kinds);
                }
            });
    }

    /// One heading and the tiles under it, or nothing when there are none.
    fn group(
        &mut self,
        ui: &mut Ui,
        open: &Open,
        editing: &mut Editing,
        heading: &str,
        kinds: &[AssetKind],
    ) {
        // Recomputed each frame, which is a map over the pool and no file
        // access: what `refresh` caches is the part that asks the disk.
        let mut entries = Vec::new();
        let Self {
            status,
            unfolded,
            thumbs,
        } = self;
        for line in listing(&open.project, status)
            .into_iter()
            .filter(|line| kinds.contains(&line.row.kind))
        {
            let folded_open = unfolded.contains(&line.row.id);
            let stills = if folded_open {
                line.stills.clone()
            } else {
                Vec::new()
            };
            entries.push(Entry::Asset(line));
            entries.extend(stills.into_iter().map(Entry::Still));
        }
        if entries.is_empty() {
            return;
        }
        ui.add_space(6.0);
        ui.label(marks::subheading(heading));
        // Less the scroll bar's lane, which is not there until the tiles
        // outgrow the column: without it the column widens by a bar's width
        // the moment they do, and the panel jumps.
        let room = ui.available_width() - ui.spacing().scroll.allocated_width();
        let width = (room - GAP * (ACROSS as f32 - 1.0)) / ACROSS as f32;
        for chunk in entries.chunks(ACROSS) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for entry in chunk {
                    tile(thumbs, unfolded, ui, width, open, editing, entry);
                }
            });
        }
    }
}

/// One tile, and what clicking, dragging and right-clicking it does.
fn tile(
    thumbs: &mut Thumbs,
    unfolded: &mut HashSet<AssetId>,
    ui: &mut Ui,
    width: f32,
    open: &Open,
    editing: &mut Editing,
    entry: &Entry<'_>,
) {
    let (status, listed) = match entry {
        Entry::Asset(line) => (line.row, Some(line)),
        Entry::Still(status) => (*status, None),
    };
    let sequence = listed.is_some_and(|line| !line.stills.is_empty());
    let picture = picture_of(open, listed, status)
        .and_then(|(source, how, out)| thumbs.get(ui.ctx(), &source, how, &out));
    let mut note = note(status, listed);
    if sequence && note.is_none() {
        let count = listed.map_or(0, |line| line.stills.len());
        note = Some((format!("{count} photos"), false));
    }
    let face = Face {
        name: status.id.as_str(),
        kind: status.kind,
        picked: editing.highlighted.as_ref() == Some(&status.id),
        note,
        unmade: matches!(status.health, AssetHealth::Awaiting(_)),
        picture,
        unfolded: sequence.then(|| unfolded.contains(&status.id)),
    };
    let drawn = tile::show(ui, width, &face);
    let picked = face.picked;
    let id = &status.id;
    let mut hover = used_by(status);
    if matches!(entry, Entry::Still(_)) {
        if status.clip_count > 0 {
            hover = format!("{hover} — also used on its own");
        }
    } else {
        hover = format!("{hover} — drag onto a track to place it, right-click to remove it");
    }
    let tile = drawn.tile.on_hover_text(hover);
    if drawn.arrow_clicked && !unfolded.remove(id) {
        unfolded.insert(id.clone());
    } else if tile.clicked() && !drawn.arrow_clicked {
        // Clicking the highlighted one again clears it, so there is a way back
        // to seeing the timeline plainly without hunting for a "none" control.
        editing.highlighted = (!picked).then(|| id.clone());
    }
    // A still is placed and reordered through its sequence: no drag, no removal.
    if matches!(entry, Entry::Asset(_)) {
        // The payload is the id, because that is the only thing a clip ever
        // refers to an asset by — see `timeline::drop`.
        tile.dnd_set_drag_payload(id.clone());
        if tile.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if tile.secondary_clicked() {
            editing.asking = Some(Asking::about(Removal::Asset(id.clone())));
        }
    }
}

/// Which kinds sit under which heading, in the order the panel lists them.
///
/// By what a thing *is*, not by what state it is in: someone looking for the
/// music knows it is sound, and does not know whether it has been generated.
///
/// That includes generated media (#1201): a narration line or a baked score is
/// sound, a generated still or shot is a picture. A heading naming a state
/// ("not made yet") told people that paid-for media did not exist, so there is
/// none; an asset that really is not made (`sketch`, `queued`, `stale`) says so
/// on its own tile instead, as a tag.
const GROUPS: &[(&str, &[AssetKind])] = &[
    (
        "PICTURE",
        &[
            AssetKind::Video,
            AssetKind::Image,
            AssetKind::ImageSequence,
            AssetKind::GeneratedVideo,
            AssetKind::GeneratedImage,
        ],
    ),
    (
        "SOUND",
        &[
            AssetKind::Audio,
            AssetKind::GeneratedAudio,
            AssetKind::SynthAudio,
        ],
    ),
    (
        "TITLES",
        &[
            AssetKind::Text,
            AssetKind::Html,
            AssetKind::Color,
            AssetKind::Shape,
            AssetKind::Icon,
            AssetKind::Group,
        ],
    ),
];

/// What a tile's picture is taken from, and where it is kept: the asset's own
/// file, or a sequence's first still. `None` for what is drawn instead, and
/// for a file that is not there to take one from.
fn picture_of(
    open: &Open,
    listed: Option<&Listed<'_>>,
    status: &AssetStatus,
) -> Option<(
    std::path::PathBuf,
    scorsese_render::frames::Thumbnail,
    std::path::PathBuf,
)> {
    let from = listed
        .and_then(|line| line.stills.first().copied())
        .unwrap_or(status);
    if !matches!(from.health, AssetHealth::Ok | AssetHealth::Unprobed) {
        return None;
    }
    let asset = open.project.asset(&from.id)?;
    let how = thumbs::how(asset.kind)?;
    let source = open.root.join(asset.path.as_ref()?.as_str());
    Some((source, how, thumbs::cached(&open.root, asset)))
}

/// The tag a tile wears: its state when that is worth a glance, and whether
/// it is a fault. A made, healthy asset wears none.
fn note(status: &AssetStatus, listed: Option<&Listed<'_>>) -> Option<(String, bool)> {
    let ailing = listed.map_or(0, |line| {
        line.stills
            .iter()
            .filter(|still| still.health.needs_attention())
            .count()
    });
    if ailing > 0 && !status.health.needs_attention() {
        return Some((format!("{ailing} need a look"), true));
    }
    let text = match &status.health {
        AssetHealth::Awaiting(state) => state.to_string(),
        AssetHealth::Missing => "missing".to_owned(),
        AssetHealth::HashMismatch { .. } => "changed".to_owned(),
        AssetHealth::Unreadable(_) => "unreadable".to_owned(),
        // Present, but nobody has asked ffprobe how long it is — the window
        // probes on open, so seeing this usually means ffprobe was not found.
        AssetHealth::Unprobed => "not probed".to_owned(),
        AssetHealth::Inline | AssetHealth::Ok => return None,
    };
    Some((text, status.health.needs_attention()))
}

/// How many clips use this asset — including none, which is what `assets gc`
/// would collect, unless something else names it (a shot a still starts from,
/// a picture a page uses): then it says what, the way `scorsese assets` does
/// (#1166), rather than implying the asset is unused.
fn used_by(status: &AssetStatus) -> String {
    match status.clip_count {
        0 => match &status.named_by {
            Some(named) => format!("used by no clip, {named}"),
            None => "used by no clip".to_owned(),
        },
        1 => "used by 1 clip".to_owned(),
        many => format!("used by {many} clips"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::NamedBy;

    fn status(clip_count: usize, named_by: Option<NamedBy>) -> AssetStatus {
        AssetStatus {
            id: AssetId::new("still"),
            kind: AssetKind::Image,
            health: AssetHealth::Ok,
            clip_count,
            named_by,
            sequence: None,
        }
    }

    #[test]
    fn an_asset_a_page_names_is_not_said_to_be_unused() {
        let named = Some(NamedBy::Page(AssetId::new("card")));
        assert_eq!(
            used_by(&status(0, named)),
            "used by no clip, named by page `card`"
        );
        assert_eq!(used_by(&status(0, None)), "used by no clip");
        assert_eq!(used_by(&status(2, None)), "used by 2 clips");
    }
}
