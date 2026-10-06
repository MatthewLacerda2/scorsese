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
//! **A drop never needs a track made first** (#771). Where no lane there can
//! take the asset — the timeline is empty, the pointer is below the last lane,
//! or the lane under it carries the other kind — the drop is
//! `placing::place_on_new_track` instead: a lane of the kind the asset needs,
//! made with the clip on it, as one edit.
//!
//! The drop is tried on every frame it hovers, not only when it lands, so the
//! ghost it draws is already the answer: the kind's colour where the clip would
//! go — on a new lane, drawn where that lane will be — or the alert colour and
//! the reason where the project would refuse it, such as a clip landing on one
//! already there.

use egui::{Pos2, Rect, Response, Stroke, StrokeKind, Ui};
use scorsese_core::{
    AssetId, AssetKind, ClipId, Fps, Frames, PlaceError, Placement, Project, Track, TrackId,
    TrackKind, placing,
};

use super::drag::{SNAP, snap::Targets};
use super::{Timeline, lanes, ruler};
use crate::editing::Editing;
use crate::project::Open;
use crate::theme::{ROUND_SM, palette};

/// How long a clip runs when its asset has no length to give it: a still, a
/// title, a colour, a shot nobody has generated yet. Five seconds, which is
/// what an editor gives a still — long enough to read, short enough to trim.
const UNMEASURED_SECONDS: f64 = 5.0;

/// Which lane a drop goes on.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Onto {
    /// The lane under the pointer, which takes the asset's kind.
    Track(TrackId),
    /// A new lane of the kind the asset needs.
    NewTrack,
}

/// Where a drop would land.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Landing {
    onto: Onto,
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
        let landing = self.landing(&open.project, &asset, area, at, editing.playhead)?;
        let outcome = place(&open.project, &asset, &landing);
        self.trouble = outcome.as_ref().err().cloned();
        let rect = self.ghost_rect(&open.project, &landing, outcome.as_ref().ok(), area);

        if response.dnd_release_payload::<AssetId>().is_some() {
            if let Ok((placed, clip, _)) = outcome {
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

    /// Where a drop at `at` would land — or nothing when the pointer is
    /// neither over a lane nor below the last one.
    fn landing(
        &self,
        project: &Project,
        asset: &AssetId,
        area: Rect,
        at: Pos2,
        playhead: Frames,
    ) -> Option<Landing> {
        if !area.contains(at) {
            return None;
        }
        let top = area.top() + ruler::HEIGHT;
        let lane = lanes::lane_at(project, area, top, at.y).map(|(track, _)| track);
        let below = at.y >= top + lanes::height(project);
        let onto = onto(project, asset, lane, below)?;
        let duration = duration_of(project, asset);
        let pointed = self.view.frame_at(at.x - area.left());
        let start = snapped(
            project,
            playhead,
            pointed,
            duration,
            self.view.frames_in(SNAP),
        );
        Some(Landing {
            onto,
            start,
            duration,
        })
    }

    /// The rectangle a drop is drawn in: on its lane, and for a new lane where
    /// that lane will be once it is made — read from the placed project's own
    /// layout, so the preview and the result cannot disagree. A new lane that
    /// would be refused is drawn just below the last one.
    fn ghost_rect(
        &self,
        project: &Project,
        landing: &Landing,
        placed: Option<&(Project, ClipId, TrackId)>,
        area: Rect,
    ) -> Rect {
        let top = area.top() + ruler::HEIGHT;
        let (laid, track) = match (placed, &landing.onto) {
            (Some((after, _, track)), _) => (after, Some(track)),
            (None, Onto::Track(track)) => (project, Some(track)),
            (None, Onto::NewTrack) => (project, None),
        };
        let offset = lanes::laid_out(laid)
            .into_iter()
            .find(|(lane, _)| Some(&lane.id) == track)
            .map_or_else(|| lanes::height(project) + lanes::GAP, |(_, at)| at);
        let lane = lanes::lane_rect(area, top, offset);
        Rect::from_min_size(
            egui::pos2(area.left() + self.view.offset_of(landing.start), lane.top()),
            egui::vec2(self.view.width_of(landing.duration).max(2.0), lane.height()),
        )
    }
}

/// Which lane a drop goes on, given the lane under the pointer (if any) and
/// whether the pointer is below the last one. A lane carrying the asset's kind
/// takes it; one carrying the other kind, the space below the last lane and an
/// empty timeline all make a new lane. In the gap between two lanes, nothing.
fn onto(project: &Project, asset: &AssetId, lane: Option<&Track>, below: bool) -> Option<Onto> {
    let wants = project
        .asset(asset)
        .and_then(|found| TrackKind::taking(found.kind));
    match lane {
        Some(track) if wants == Some(track.kind) => Some(Onto::Track(track.id.clone())),
        Some(_) => Some(Onto::NewTrack),
        None => below.then_some(Onto::NewTrack),
    }
}

/// Draws the clip a drop would place.
pub(super) fn ghost(painter: &egui::Painter, ghost: &Ghost) {
    let colour = match (ghost.takes, ghost.kind) {
        (true, Some(kind)) => palette::of_kind(kind),
        (true, None) => palette::of(painter.ctx()).foreground,
        (false, _) => palette::of(painter.ctx()).destructive,
    };
    painter.rect_filled(ghost.rect, ROUND_SM, colour.gamma_multiply(0.35));
    painter.rect_stroke(
        ghost.rect,
        ROUND_SM,
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

/// The project with a clip of `asset` placed at `landing`, the new clip's id
/// and the track it went on — or the first reason it cannot be, in one line
/// for the timeline's note.
fn place(
    project: &Project,
    asset: &AssetId,
    landing: &Landing,
) -> Result<(Project, ClipId, TrackId), String> {
    let mut placed = project.clone();
    let duration = Some(landing.duration);
    let outcome = match &landing.onto {
        Onto::Track(track) => {
            let placement = Placement {
                asset: asset.clone(),
                track: track.clone(),
                start: landing.start,
                duration,
                source_in: Frames::ZERO,
                id: None,
            };
            placing::place(&mut placed, &placement).map(|clip| (track.clone(), clip))
        }
        Onto::NewTrack => placing::place_on_new_track(&mut placed, asset, landing.start, duration),
    };
    match outcome {
        Ok((track, clip)) => Ok((placed, clip.id, track)),
        Err(PlaceError::Refused(errors)) => Err(errors
            .into_vec()
            .into_iter()
            .next()
            .map_or_else(|| "refused".to_owned(), |problem| problem.to_string())),
        Err(other) => Err(other.to_string()),
    }
}

#[cfg(test)]
mod tests;
