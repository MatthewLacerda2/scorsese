//! Removing an asset or a track from the window: a plain confirm that lists the
//! clips going with it, then one call naming exactly those (#396).
//!
//! Asked for by a right-click — on an asset's name in the project files, or on
//! a lane's head in the timeline — and answered here, in one floating window
//! over the panels, the way the generate dialog is. The list is worked out from
//! the document the window holds *now*, every frame the confirm is up, so a
//! change from outside while it is open is a list that changes in front of the
//! person rather than one that goes stale behind their back. The core call is
//! the same one `asset_remove` and `track_remove` make, and it refuses any list
//! but the exact one, so what the confirm showed is what goes.

use std::collections::BTreeSet;

use egui::{Align2, Context, RichText, Window};
use scorsese_core::{AssetId, ClipId, Project, TrackId, authoring};

use crate::project::Open;
use crate::theme::palette;

/// What a right-click asked to remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Removal {
    /// An asset, and every clip that shows it.
    Asset(AssetId),
    /// A track, and every clip on it.
    Track(TrackId),
}

/// A confirm on screen: what it is about, and why the last attempt failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asking {
    what: Removal,
    trouble: Option<String>,
}

impl Asking {
    /// A confirm about to be put up.
    pub(crate) fn about(what: Removal) -> Self {
        Self {
            what,
            trouble: None,
        }
    }
}

impl Removal {
    /// The clips that would go, or `None` when the thing itself is gone.
    fn clips(&self, project: &Project) -> Option<BTreeSet<ClipId>> {
        match self {
            Self::Asset(asset) => {
                project.asset(asset)?;
                Some(
                    project
                        .every_clip()
                        .filter(|(_, clip)| &clip.asset == asset)
                        .map(|(_, clip)| clip.id.clone())
                        .collect(),
                )
            }
            Self::Track(track) => project
                .every_track()
                .find(|lane| &lane.id == track)
                .map(|lane| lane.clips.iter().map(|clip| clip.id.clone()).collect()),
        }
    }

    /// The question, as the confirm puts it.
    fn question(&self, going: usize) -> String {
        let what = match self {
            Self::Asset(asset) => format!("“{asset}” from the project"),
            Self::Track(track) => format!("the track “{track}”"),
        };
        let tail = match (self, going) {
            (_, 0) => return format!("Remove {what}?"),
            (Self::Asset(_), 1) => "The clip that uses it will be deleted too:".to_owned(),
            (Self::Asset(_), many) => format!("These {many} clips use it and will be deleted too:"),
            (Self::Track(_), 1) => "The clip on it will be deleted too:".to_owned(),
            (Self::Track(_), many) => format!("The {many} clips on it will be deleted too:"),
        };
        format!("Remove {what}? {tail}")
    }

    /// Removes it and the clips named, all or nothing.
    fn apply(&self, project: &mut Project, clips: &BTreeSet<ClipId>) -> Result<(), String> {
        let done = match self {
            Self::Asset(asset) => authoring::remove_asset(project, asset, clips).map(drop),
            Self::Track(track) => authoring::remove_track(project, track, clips).map(drop),
        };
        done.map_err(|why| why.to_string())
    }
}

/// Draws the confirm while one is asked for, and carries out a yes. Answers
/// whether the document changed, so the panels that cache it read it again.
pub(crate) fn show(ctx: &Context, open: &mut Open, asking: &mut Option<Asking>) -> bool {
    let Some(current) = asking.as_mut() else {
        return false;
    };
    // Gone from under the confirm — an assistant removed it first — is nothing
    // left to ask about.
    let Some(clips) = current.what.clips(&open.project) else {
        *asking = None;
        return false;
    };
    let mut answer = None;
    Window::new("Remove")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(current.what.question(clips.len()));
            for clip in &clips {
                ui.label(RichText::new(format!("  {clip}")).monospace());
            }
            if let Some(trouble) = &current.trouble {
                ui.label(
                    RichText::new(trouble)
                        .small()
                        .color(palette::of(ui.ctx()).warning),
                );
            }
            ui.separator();
            ui.horizontal(|ui| {
                let allowed = !open.read_only();
                if ui
                    .add_enabled(allowed, egui::Button::new("Remove"))
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    answer = Some(false);
                }
            });
        });
    match answer {
        Some(true) => {
            let mut proposed = open.project.clone();
            // Saved before it is shown, as every other edit in the window is.
            let saved = current
                .what
                .apply(&mut proposed, &clips)
                .and_then(|()| proposed.save(&open.root).map_err(|why| why.to_string()));
            match saved {
                Ok(()) => {
                    open.project = proposed;
                    *asking = None;
                    true
                }
                Err(why) => {
                    current.trouble = Some(why);
                    false
                }
            }
        }
        Some(false) => {
            *asking = None;
            false
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        Project::from_json(
            r#"{ "schema_version": 45, "name": "T", "timeline_fps": { "num": 30, "den": 1 },
              "assets": [ { "id": "title", "kind": "text", "text": "T" } ],
              "tracks": [ { "id": "v1", "kind": "video", "clips": [
                { "id": "c1", "asset": "title", "start": 0, "duration": 30 },
                { "id": "c2", "asset": "title", "start": 30, "duration": 30 } ] },
                { "id": "a1", "kind": "audio", "clips": [] } ] }"#,
        )
        .expect("the fixture is a project")
    }

    #[test]
    fn the_confirm_lists_what_goes_and_a_yes_removes_exactly_that() {
        let mut project = project();
        let asset = Removal::Asset(AssetId::new("title"));
        let clips = asset.clips(&project).expect("the title is there");
        assert_eq!(clips.len(), 2);
        assert!(asset.question(2).contains("These 2 clips use it"));
        asset
            .apply(&mut project, &clips)
            .expect("the list is exact");
        assert!(project.asset(&AssetId::new("title")).is_none());
        assert_eq!(project.clips().count(), 0);
        assert!(asset.clips(&project).is_none(), "nothing left to ask about");
    }

    #[test]
    fn an_empty_lane_is_asked_about_plainly() {
        let track = Removal::Track(TrackId::new("a1"));
        assert_eq!(track.question(0), "Remove the track “a1”?");
        assert_eq!(
            track.question(1),
            "Remove the track “a1”? The clip on it will be deleted too:"
        );
    }
}
