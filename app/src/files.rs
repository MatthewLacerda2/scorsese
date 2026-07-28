//! The project files panel: what the edit is made of.
//!
//! The same answer `scorsese assets` gives, through the same
//! [`asset_status`] — so the window and the command line can never disagree
//! about what is in the pool or what state it is in.
//!
//! Grouped by what a thing *is*, because that is how someone looks for one:
//! footage, sound, titles, and the things that do not exist yet.

use egui::{Grid, RichText, ScrollArea, Ui};
use scorsese_core::{AssetHealth, AssetKind, AssetStatus, HashCheck, Project, asset_status};

use crate::editing::Editing;
use crate::project::Open;

/// The panel's own state: the pool as it was last looked at.
#[derive(Debug, Default)]
pub(crate) struct Files {
    status: Vec<AssetStatus>,
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
    }

    /// Draws the panel.
    pub(crate) fn show(&mut self, ui: &mut Ui, open: &Open, editing: &mut Editing) {
        ui.horizontal(|ui| {
            ui.heading("Project files");
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
            ui.label(RichText::new("nothing imported yet").weak().italics());
            return;
        }

        ScrollArea::vertical().show(ui, |ui| {
            for (heading, kinds) in GROUPS {
                self.group(ui, &open.project, editing, heading, kinds);
            }
        });
    }

    /// One heading and the assets under it, or nothing when there are none.
    fn group(
        &self,
        ui: &mut Ui,
        project: &Project,
        editing: &mut Editing,
        heading: &str,
        kinds: &[AssetKind],
    ) {
        let rows: Vec<&AssetStatus> = self
            .status
            .iter()
            .filter(|status| kinds.contains(&status.kind))
            .collect();
        if rows.is_empty() {
            return;
        }
        ui.add_space(6.0);
        ui.label(RichText::new(heading).strong().small());
        Grid::new(heading)
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for status in rows {
                    row(ui, project, editing, status);
                }
            });
    }
}

/// Which kinds sit under which heading, in the order the panel lists them.
///
/// By what a thing *is*, not by what state it is in: someone looking for the
/// music knows it is sound, and does not know whether it has been generated.
const GROUPS: &[(&str, &[AssetKind])] = &[
    ("PICTURE", &[AssetKind::Video, AssetKind::Image]),
    ("SOUND", &[AssetKind::Audio]),
    ("TITLES", &[AssetKind::Text]),
    (
        "NOT MADE YET",
        &[
            AssetKind::GeneratedVideo,
            AssetKind::GeneratedAudio,
            AssetKind::SynthAudio,
        ],
    ),
];

/// One asset: its id, what is true of it, and whether it is highlighted.
///
/// Clicking highlights the asset's clips in the timeline — the answer to
/// "where does this actually get used?", which the table alone cannot give.
fn row(ui: &mut Ui, project: &Project, editing: &mut Editing, status: &AssetStatus) {
    let picked = editing.highlighted.as_ref() == Some(&status.id);
    if ui
        .selectable_label(picked, status.id.as_str())
        .on_hover_text(used_by(status))
        .clicked()
    {
        // Clicking the highlighted one again clears it, so there is a way back
        // to seeing the timeline plainly without hunting for a "none" control.
        editing.highlighted = (!picked).then(|| status.id.clone());
    }
    ui.label(note(project, status));
    ui.end_row();
}

/// The short right-hand note: what is worth knowing at a glance.
fn note(project: &Project, status: &AssetStatus) -> RichText {
    let text = match &status.health {
        AssetHealth::Awaiting(state) => format!("{state:?}").to_lowercase(),
        AssetHealth::Missing => "file missing".to_owned(),
        AssetHealth::HashMismatch { .. } => "changed on disk".to_owned(),
        AssetHealth::Unreadable(_) => "unreadable".to_owned(),
        // Present but never probed: not a fault, and not worth shouting
        // about — it only means nobody has asked ffprobe how long it is.
        AssetHealth::Unprobed => "not probed".to_owned(),
        AssetHealth::Inline | AssetHealth::Ok => recipe_of(project, status),
    };
    let text = RichText::new(text).small();
    if status.health.needs_attention() {
        text.color(egui::Color32::from_rgb(220, 140, 90))
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
