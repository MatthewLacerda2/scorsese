//! The project files panel: what the edit is made of.
//!
//! The same answer `scorsese assets` gives, through the same
//! [`asset_status`] — so the window and the command line can never disagree
//! about what is in the pool or what state it is in.
//!
//! Grouped by what a thing *is*, because that is how someone looks for one:
//! footage, sound, titles, and the things that do not exist yet.
//!
//! It is also where a clip comes from: an asset's name dragged onto a lane of
//! the timeline places a clip of it there. Right-clicked, it asks to remove the
//! asset and the clips using it ([`crate::removing`]).
//!
//! An image sequence is one row, its stills folded under it behind a
//! disclosure arrow (#684): a 400-photo timelapse would otherwise be 401 rows,
//! 400 of which nobody drags anywhere on their own. Unfolded, the stills are
//! there to be *seen* — which file, whether it is healthy, whether a clip also
//! shows it alone — and reordering them stays a sentence to the assistant.

use std::collections::HashSet;

use egui::{Grid, RichText, ScrollArea, Sense, Ui, vec2};
use scorsese_core::{
    AssetHealth, AssetId, AssetKind, AssetStatus, HashCheck, Listed, Project, asset_status, listing,
};

use crate::editing::Editing;
use crate::project::Open;
use crate::removing::{Asking, Removal};
use crate::theme::{ROUND_SM, marks, palette};

/// How big the colour chip beside an asset's name is.
///
/// A square rather than a swatch the width of the row: the chip is an *index*
/// into the timeline's colours, and it only has to be big enough to tell a blue
/// from a green at a glance.
const CHIP: f32 = 8.0;

/// The panel's own state: the pool as it was last looked at.
#[derive(Debug, Default)]
pub(crate) struct Files {
    status: Vec<AssetStatus>,
    /// The sequences someone has unfolded. Folded is the default, so a
    /// project opens with every sequence as one row.
    unfolded: HashSet<AssetId>,
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

        // No floor under its height. egui's default makes a scroll area at
        // least 64 points tall whatever room is left, and in a short window
        // that floor pushed the side column past the bottom of the window
        // (#681): a list that shrinks to a sliver is still reachable by
        // scrolling, and a column drawn off-screen is not.
        ScrollArea::vertical()
            .min_scrolled_height(0.0)
            .show(ui, |ui| {
                for (heading, kinds) in GROUPS {
                    self.group(ui, &open.project, editing, heading, kinds);
                }
            });
    }

    /// One heading and the assets under it, or nothing when there are none.
    fn group(
        &mut self,
        ui: &mut Ui,
        project: &Project,
        editing: &mut Editing,
        heading: &str,
        kinds: &[AssetKind],
    ) {
        // Recomputed each frame, which is a map over the pool and no file
        // access: what `refresh` caches is the part that asks the disk.
        let lines: Vec<Listed<'_>> = listing(project, &self.status)
            .into_iter()
            .filter(|line| kinds.contains(&line.row.kind))
            .collect();
        if lines.is_empty() {
            return;
        }
        let unfolded = &mut self.unfolded;
        ui.add_space(6.0);
        ui.label(marks::subheading(heading));
        Grid::new(heading)
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                for line in lines {
                    if line.stills.is_empty() {
                        chip(ui, line.row.kind);
                        row(ui, project, editing, line.row, line.row.id.as_str());
                    } else {
                        sequence(ui, project, editing, &line, unfolded);
                    }
                }
            });
    }
}

/// Which kinds sit under which heading, in the order the panel lists them.
///
/// By what a thing *is*, not by what state it is in: someone looking for the
/// music knows it is sound, and does not know whether it has been generated.
const GROUPS: &[(&str, &[AssetKind])] = &[
    (
        "PICTURE",
        &[AssetKind::Video, AssetKind::Image, AssetKind::ImageSequence],
    ),
    ("SOUND", &[AssetKind::Audio]),
    (
        "TITLES",
        &[AssetKind::Text, AssetKind::Html, AssetKind::Color],
    ),
    (
        "NOT MADE YET",
        &[
            AssetKind::GeneratedVideo,
            AssetKind::GeneratedImage,
            AssetKind::GeneratedAudio,
            AssetKind::SynthAudio,
        ],
    ),
];

/// One asset: its id, what is true of it, and whether it is highlighted.
///
/// Clicking highlights the asset's clips in the timeline — the answer to
/// "where does this actually get used?", which the table alone cannot give.
fn row(ui: &mut Ui, project: &Project, editing: &mut Editing, status: &AssetStatus, label: &str) {
    let picked = editing.highlighted.as_ref() == Some(&status.id);
    let name = ui
        .selectable_label(picked, label)
        // Draggable as well as clickable: dragging a name onto a lane is how a
        // clip of it gets placed — see `timeline::drop`. The payload is the id,
        // because that is the only thing a clip ever refers to an asset by.
        .interact(Sense::drag())
        .on_hover_text(format!(
            "{} — drag onto a track to place it, right-click to remove it",
            used_by(status)
        ));
    name.dnd_set_drag_payload(status.id.clone());
    if name.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if name.clicked() {
        // Clicking the highlighted one again clears it, so there is a way back
        // to seeing the timeline plainly without hunting for a "none" control.
        editing.highlighted = (!picked).then(|| status.id.clone());
    }
    if name.secondary_clicked() {
        editing.asking = Some(Asking::about(Removal::Asset(status.id.clone())));
    }
    let warning = palette::of(ui.ctx()).warning;
    ui.label(note(project, status, warning));
    ui.end_row();
}

/// A sequence's own row — the arrow in the chip's column, its name saying how
/// many photos it plays — and, unfolded, one compact row per still under it.
///
/// Folded, a still that needs attention would be invisible, so the sequence's
/// row says how many do: a missing photo is found by unfolding, not by luck.
fn sequence(
    ui: &mut Ui,
    project: &Project,
    editing: &mut Editing,
    line: &Listed<'_>,
    unfolded: &mut HashSet<AssetId>,
) {
    let id = &line.row.id;
    let open = unfolded.contains(id);
    let arrow = ui
        .add(
            egui::Button::new(if open { "⏷" } else { "⏵" })
                .small()
                .frame(false),
        )
        .on_hover_text(if open {
            "Fold the photos away"
        } else {
            "Show its photos"
        });
    if arrow.clicked() && !unfolded.remove(id) {
        unfolded.insert(id.clone());
    }
    let count = line.stills.len();
    let photos = if count == 1 { "photo" } else { "photos" };
    let picked = editing.highlighted.as_ref() == Some(id);
    let name = ui
        .selectable_label(picked, format!("{id} — {count} {photos}"))
        .interact(Sense::drag())
        .on_hover_text(format!(
            "{} — drag onto a track to place it, right-click to remove it",
            used_by(line.row)
        ));
    name.dnd_set_drag_payload(id.clone());
    if name.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if name.clicked() {
        editing.highlighted = (!picked).then(|| id.clone());
    }
    if name.secondary_clicked() {
        editing.asking = Some(Asking::about(Removal::Asset(id.clone())));
    }
    let warning = palette::of(ui.ctx()).warning;
    let ailing = line
        .stills
        .iter()
        .filter(|still| still.health.needs_attention())
        .count();
    if ailing > 0 && !line.row.health.needs_attention() {
        let need = if ailing == 1 { "needs" } else { "need" };
        ui.label(
            RichText::new(format!("{ailing} {need} a look"))
                .small()
                .color(warning),
        );
    } else {
        ui.label(note(project, line.row, warning));
    }
    ui.end_row();
    if open {
        for still in &line.stills {
            stilled(ui, project, editing, still, warning);
        }
    }
}

/// One still under its unfolded sequence: for seeing, not for dragging.
///
/// Clicking it highlights the clips that show it on its own, the same answer
/// any other row gives; it carries no drag and no removal, because a still is
/// placed and reordered through its sequence.
fn stilled(
    ui: &mut Ui,
    project: &Project,
    editing: &mut Editing,
    still: &AssetStatus,
    warning: egui::Color32,
) {
    ui.label("");
    let picked = editing.highlighted.as_ref() == Some(&still.id);
    let name = ui
        .selectable_label(picked, RichText::new(format!("  {}", still.id)).small())
        .on_hover_text(used_by(still));
    if name.clicked() {
        editing.highlighted = (!picked).then(|| still.id.clone());
    }
    let mut text = note(project, still, warning);
    if !still.health.needs_attention() && still.clip_count > 0 {
        text = RichText::new("also used on its own").small().weak();
    }
    ui.label(text);
    ui.end_row();
}

/// The colour that says what this asset is — the same one its clips are drawn
/// in, which is the whole of what makes the pool and the timeline one picture
/// rather than two lists.
fn chip(ui: &mut Ui, kind: AssetKind) {
    let (rect, _) = ui.allocate_exact_size(vec2(CHIP, CHIP), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, ROUND_SM, palette::of_kind(kind));
}

/// The short right-hand note: what is worth knowing at a glance.
fn note(project: &Project, status: &AssetStatus, warning: egui::Color32) -> RichText {
    let text = match &status.health {
        AssetHealth::Awaiting(state) => format!("{state:?}").to_lowercase(),
        AssetHealth::Missing => "file missing".to_owned(),
        AssetHealth::HashMismatch { .. } => "changed on disk".to_owned(),
        AssetHealth::Unreadable(_) => "unreadable".to_owned(),
        // Present, but nobody has asked ffprobe how long it is — so anything
        // that needs the source's own length skips it. The window probes on
        // open, so seeing this usually means ffprobe could not be found.
        AssetHealth::Unprobed => "not probed".to_owned(),
        AssetHealth::Inline | AssetHealth::Ok => recipe_of(project, status),
    };
    let text = RichText::new(text).small();
    if status.health.needs_attention() {
        text.color(warning)
    } else {
        text.weak()
    }
}

/// A synthesised asset's recipe, which is the one relationship the assets
/// table does not show on its own: the file you edit to change this sound.
fn recipe_of(project: &Project, status: &AssetStatus) -> String {
    project
        .asset(&status.id)
        .and_then(|asset| asset.recipe.as_ref())
        .map(|recipe| {
            recipe
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or(recipe.as_str())
                .to_owned()
        })
        .unwrap_or_default()
}

/// How many clips use this asset — including none, which is what `assets gc`
/// would collect.
fn used_by(status: &AssetStatus) -> String {
    match status.clip_count {
        0 => "used by no clip".to_owned(),
        1 => "used by 1 clip".to_owned(),
        many => format!("used by {many} clips"),
    }
}
