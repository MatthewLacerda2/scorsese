//! Placing a clip: an asset's name dragged out of the project files and let go
//! over a lane.
//!
//! The payload is the asset's id — set by the files panel the moment its name
//! is dragged — and nothing else, because an id is all a clip ever refers to an
//! asset by. What happens to it here is `scorsese_core::placing::place`, the
//! same call `place_clip` makes for an assistant: a new clip on an existing
//! track, validated whole, all or nothing. The window decides only *where* —
//! the lane under the pointer and the frame under it, pulled onto a nearby cut
//! the way a dragged clip is — and *how long* when the asset cannot say.
//!
//! The drop is tried on every frame it hovers, not only when it lands, so the
//! ghost it draws is already the answer: the kind's colour where the clip would
//! go, or the alert colour and the reason where the project would refuse it —
//! a sound over a picture track, or a clip landing on one already there.

use egui::{Pos2, Rect, Response, Stroke, StrokeKind, Ui};
use scorsese_core::{
    AssetId, AssetKind, ClipId, Fps, Frames, PlaceError, Placement, Project, TrackId, placing,
};

use super::drag::{SNAP, snap::Targets};
use super::{Timeline, lanes, ruler};
use crate::editing::Editing;
use crate::project::Open;
use crate::theme::{ROUND, palette};

/// How long a clip runs when its asset has no length to give it: a still, a
/// title, a colour, a shot nobody has generated yet. Five seconds, which is
/// what an editor gives a still — long enough to read, short enough to trim.
const UNMEASURED_SECONDS: f64 = 5.0;

/// Where a drop would land.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Landing {
    track: TrackId,
    start: Frames,
    duration: Frames,
}

/// What the timeline draws while an asset hovers over a lane.
#[derive(Debug)]
pub(super) struct Ghost {
    /// Where the clip would sit.
    pub(super) rect: Rect,
    /// Whether the project would take it — decides the colour.
    pub(super) takes: bool,
    /// What the asset is, for the colour when it does.
    pub(super) kind: Option<AssetKind>,
}

impl Timeline {
    /// Answers an asset being dragged over the timeline: a ghost while it
    /// hovers over a lane, and a placed clip when it is let go there.
    pub(super) fn dropping(
        &mut self,
        ui: &Ui,
        response: &Response,
        area: Rect,
        open: &mut Open,
        editing: &mut Editing,
    ) -> Option<Ghost> {
        // Disabled on a read-only project, like every other edit on this panel.
        if !ui.is_enabled() {
            return None;
        }
        let asset = response.dnd_hover_payload::<AssetId>()?;
        let at = ui.input(|input| input.pointer.latest_pos())?;
        let (landing, rect) = self.landing(&open.project, &asset, area, at, editing.playhead)?;
        let outcome = place(&open.project, &asset, &landing);
        self.trouble = outcome.as_ref().err().cloned();

        if response.dnd_release_payload::<AssetId>().is_some() {
            if let Ok((placed, clip)) = outcome {
                // Saved before it is shown, as every other edit here is.
                match placed.save(&open.root) {
                    Ok(()) => {
                        open.project = placed;
                        // The inspector looks at what the hand just put down.
                        editing.selected = [clip].into();
                    }
                    Err(why) => self.trouble = Some(why.to_string()),
                }
            }
            return None;
        }
        Some(Ghost {
            rect,
            takes: outcome.is_ok(),
            kind: open.project.asset(&asset).map(|found| found.kind),
        })
    }

    /// Where a drop at `at` would land, and the rectangle it would be drawn in
    /// — or nothing when the pointer is not over a lane.
    fn landing(
        &self,
        project: &Project,
        asset: &AssetId,
        area: Rect,
        at: Pos2,
        playhead: Frames,
    ) -> Option<(Landing, Rect)> {
        if !area.contains(at) {
            return None;
        }
        let top = area.top() + ruler::HEIGHT;
        let (track, lane) = lanes::lane_at(project, area, top, at.y)?;
        let duration = duration_of(project, asset);
        let pointed = self.view.frame_at(at.x - area.left());
        let start = snapped(
            project,
            playhead,
            pointed,
            duration,
            self.view.frames_in(SNAP),
        );
        let left = area.left() + self.view.offset_of(start);
        let rect = Rect::from_min_size(
            egui::pos2(left, lane.top()),
            egui::vec2(self.view.width_of(duration).max(2.0), lane.height()),
        );
        let landing = Landing {
            track: track.id.clone(),
            start,
            duration,
        };
        Some((landing, rect))
    }
}

/// Draws the clip a drop would place.
pub(super) fn ghost(painter: &egui::Painter, ghost: &Ghost) {
    let colour = match (ghost.takes, ghost.kind) {
        (true, Some(kind)) => palette::of_kind(kind),
        (true, None) => palette::ACCENT,
        (false, _) => palette::ALERT,
    };
    painter.rect_filled(ghost.rect, ROUND, colour.gamma_multiply(0.35));
    painter.rect_stroke(
        ghost.rect,
        ROUND,
        Stroke::new(1.0, colour),
        StrokeKind::Inside,
    );
}

/// How long a clip of `asset` runs when placed: the whole of it, when it has a
/// measured length, and [`UNMEASURED_SECONDS`] when it has none.
fn duration_of(project: &Project, asset: &AssetId) -> Frames {
    let fps: Fps = project.timeline_fps;
    project
        .asset(asset)
        .and_then(|found| found.length(fps))
        .filter(|length| *length > Frames::ZERO)
        .unwrap_or_else(|| fps.frames(UNMEASURED_SECONDS))
}

/// Where a clip pointed at `pointed` starts once pulled onto a nearby cut.
///
/// Either edge may find the target — dropping a shot so its tail meets the
/// next one's head is as common as butting its head against the last one's
/// tail — and never before the start of the timeline.
fn snapped(
    project: &Project,
    playhead: Frames,
    pointed: Frames,
    duration: Frames,
    reach: Frames,
) -> Frames {
    Targets::gather(project, playhead, None)
        .nearest(&[pointed, pointed + duration], reach)
        .map_or(pointed, |snap| {
            Frames((pointed.get() as i64 + snap.shift).max(0) as u64)
        })
}

/// The project with a clip of `asset` placed at `landing`, and the new clip's
/// id — or the first reason it cannot be, in one line for the timeline's note.
fn place(
    project: &Project,
    asset: &AssetId,
    landing: &Landing,
) -> Result<(Project, ClipId), String> {
    let mut placed = project.clone();
    let placement = Placement {
        asset: asset.clone(),
        track: landing.track.clone(),
        start: landing.start,
        duration: Some(landing.duration),
        source_in: Frames::ZERO,
        id: None,
    };
    match placing::place(&mut placed, &placement) {
        Ok(clip) => Ok((placed, clip.id)),
        Err(PlaceError::Refused(errors)) => Err(errors
            .into_vec()
            .into_iter()
            .next()
            .map_or_else(|| "refused".to_owned(), |problem| problem.to_string())),
        Err(other) => Err(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{Asset, AssetKind, Clip, ProjectPath, Track, TrackKind};

    /// A 4-second shot already on `v1` at 0..120, a title with no length, a
    /// sound, and an empty audio track.
    fn project() -> Project {
        let mut project = Project::new("t", Fps::THIRTY);
        let mut shot = Asset::imported(
            AssetId::new("shot"),
            AssetKind::Video,
            ProjectPath::new("assets/shot.mp4"),
        );
        shot.media = Some(scorsese_core::MediaMetadata {
            duration_seconds: Some(4.0),
            ..Default::default()
        });
        project.assets.push(shot);
        project
            .assets
            .push(Asset::text(AssetId::new("title"), "hi"));
        project.assets.push(Asset::imported(
            AssetId::new("music"),
            AssetKind::Audio,
            ProjectPath::new("assets/m.wav"),
        ));
        let mut video = Track::new(TrackId::new("v1"), TrackKind::Video);
        video.clips.push(Clip::new(
            ClipId::new("head"),
            AssetId::new("shot"),
            Frames(0),
            Frames(120),
        ));
        project.tracks.push(video);
        project
            .tracks
            .push(Track::new(TrackId::new("a1"), TrackKind::Audio));
        project
    }

    fn landing(track: &str, start: u64, duration: u64) -> Landing {
        Landing {
            track: TrackId::new(track),
            start: Frames(start),
            duration: Frames(duration),
        }
    }

    /// A measured shot is placed whole; a title, which has no length of its
    /// own, gets five seconds rather than a refusal.
    #[test]
    fn a_placed_clip_runs_the_whole_asset_or_five_seconds() {
        let project = project();
        assert_eq!(duration_of(&project, &AssetId::new("shot")), Frames(120));
        assert_eq!(duration_of(&project, &AssetId::new("title")), Frames(150));
    }

    /// Dropped past the end of what is there, the clip lands, gets an id of
    /// its own, and is the document's rather than a copy of it.
    #[test]
    fn a_drop_on_a_free_stretch_places_a_clip() {
        let before = project();
        let (after, clip) = place(&before, &AssetId::new("shot"), &landing("v1", 120, 120))
            .expect("the stretch is free");
        assert_eq!(clip, ClipId::new("shot"));
        assert_eq!(after.tracks[0].clips.len(), 2);
        assert_eq!(
            before.tracks[0].clips.len(),
            1,
            "the window's document is untouched until saved"
        );
    }

    /// The two refusals a hand will actually meet: landing on a clip already
    /// there, and a sound dropped on a picture track.
    #[test]
    fn a_drop_the_project_would_not_survive_is_refused_with_a_reason() {
        let project = project();
        let overlap = place(&project, &AssetId::new("title"), &landing("v1", 60, 150));
        assert!(overlap.is_err());
        let wrong_lane = place(&project, &AssetId::new("music"), &landing("v1", 300, 30));
        assert!(
            wrong_lane
                .expect_err("a sound has no place on a picture track")
                .contains("music")
        );
    }

    /// Dropped a few frames short of the last clip's end, a shot is pulled
    /// onto it rather than leaving a gap that renders as a black flash.
    #[test]
    fn a_drop_near_a_cut_is_pulled_onto_it() {
        let project = project();
        let start = snapped(&project, Frames(900), Frames(124), Frames(120), Frames(8));
        assert_eq!(start, Frames(120));
        let far = snapped(&project, Frames(900), Frames(400), Frames(120), Frames(8));
        assert_eq!(far, Frames(400));
    }
}
