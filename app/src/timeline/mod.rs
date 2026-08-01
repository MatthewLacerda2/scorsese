//! The timeline: tracks and clips laid out along time, with the playhead.
//!
//! Drawn rather than assembled from widgets, because a timeline is a picture
//! of time and nothing in a widget library is shaped like one. What that buys
//! is that every position on screen comes from one transform ([`view::View`]),
//! so the ruler, the clips and the playhead cannot disagree about where a
//! frame is.

mod drag;
mod gesture;
mod lanes;
mod ruler;
mod view;

use egui::{Rect, Sense, Ui, vec2};
use scorsese_core::Project;

use crate::editing::{Editing, length};
use crate::project::Open;
use gesture::Gesture;
use view::View;

/// A frame count as a person reads a time. Lives with the view because that is
/// the one module allowed to turn frames into seconds; re-exported because the
/// transport under the preview reads out the same playhead this ruler labels,
/// and two spellings of a timecode would be two answers to the same question.
pub(crate) use view::timecode;

/// How wide the track-label gutter is.
const GUTTER: f32 = 96.0;
/// How much one notch of the wheel magnifies.
const ZOOM_STEP: f32 = 1.15;

/// The timeline's own state: where it is looking, and what a hand is doing.
#[derive(Debug, Default)]
pub(crate) struct Timeline {
    view: View,
    /// Whether the view has been fitted to the project that is open. Fitting
    /// once on open and never again — a view that re-fits itself would undo
    /// every zoom the moment anything changed.
    fitted: bool,
    /// The gesture in flight, if any.
    gesture: Option<Gesture>,
    /// Why the last edit did not take. A refused drag shows itself — the clip
    /// stops following the pointer — but *why* is the part nobody can guess,
    /// and a save that failed would otherwise be silent.
    trouble: Option<String>,
}

impl Timeline {
    /// Forgets the fit, so the next project shown gets its own.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whether a hand is on something right now.
    ///
    /// Asked by the reload: replacing the document mid-drag would yank the
    /// clip out from under the pointer, so a change that arrives during a
    /// gesture waits for it to end.
    pub(crate) fn busy(&self) -> bool {
        self.gesture.is_some()
    }

    /// Draws the timeline and handles what happens on it.
    pub(crate) fn show(&mut self, ui: &mut Ui, open: &mut Open, editing: &mut Editing) {
        let full = ui.available_rect_before_wrap();
        let (gutter, area) = split(full);
        if !self.fitted {
            self.view.fit(length(&open.project), area.width());
            self.fitted = true;
        }

        let response = ui.allocate_rect(full, Sense::click_and_drag());
        // `interact_pointer_pos` first: while a button is down it is the
        // authority on where the pointer is, and hovering gives up the moment
        // a drag leaves the panel.
        let pointer = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos());
        self.navigate(ui, area, pointer);

        let top = area.top() + ruler::HEIGHT;
        let hit = pointer
            .filter(|at| area.x_range().contains(at.x))
            .and_then(|at| lanes::hit(&open.project, area, top, self.view, at));
        // Input before paint, so a clip moved this frame is drawn where the
        // pointer left it rather than one frame behind it.
        self.act(ui, &response, area, hit.as_ref(), open, editing);

        let painter = ui.painter_at(full);
        ruler::draw(
            ui,
            &painter,
            ruler_rect(area),
            self.view,
            open.project.timeline_fps,
        );
        divider(ui, &painter, area, &open.project);
        rows(
            &lanes::Paint {
                ui,
                painter: &painter,
                project: &open.project,
                view: self.view,
                selected: &editing.selected,
                highlighted: editing.highlighted.clone(),
            },
            gutter,
            area,
        );

        self.snap_line(ui, &painter, area);
        self.note(ui, &painter, area);
        ruler::playhead(
            &painter,
            area,
            area.left() + self.view.offset_of(editing.playhead),
            ui.visuals().error_fg_color,
        );
    }

    /// Scrolling and zooming, from the wheel.
    ///
    /// Plain wheel scrolls and ctrl-wheel zooms, which is what every timeline
    /// anyone has used already does — this is not the place to be interesting.
    fn navigate(&mut self, ui: &Ui, area: Rect, pointer: Option<egui::Pos2>) {
        let Some(at) = pointer.filter(|at| area.contains(*at)) else {
            return;
        };
        let (scroll, zoom) = ui.input(|input| (input.smooth_scroll_delta, input.zoom_delta()));
        if zoom != 1.0 {
            self.view.zoom(zoom, at.x - area.left());
        } else if scroll.y != 0.0 && ui.input(|input| input.modifiers.ctrl) {
            self.view
                .zoom(ZOOM_STEP.powf(scroll.y.signum()), at.x - area.left());
        } else if scroll.x != 0.0 || scroll.y != 0.0 {
            // A vertical wheel scrolls the timeline sideways when there is no
            // horizontal axis to use: most mice have one wheel, and sideways
            // is the only direction this panel can go.
            let sideways = if scroll.x != 0.0 { scroll.x } else { scroll.y };
            self.view.scroll(-sideways);
        }
    }

    /// The line a drag has come to rest against.
    ///
    /// Only while a gesture is live and only when one was actually taken: an
    /// indicator that is always on says nothing.
    fn snap_line(&self, ui: &Ui, painter: &egui::Painter, area: Rect) {
        let Some(Gesture::Clip(held)) = &self.gesture else {
            return;
        };
        let Some(frame) = held.snapped() else {
            return;
        };
        let x = area.left() + self.view.offset_of(frame);
        if !area.x_range().contains(x) {
            return;
        }
        painter.line_segment(
            [
                egui::pos2(x, area.top() + ruler::HEIGHT),
                egui::pos2(x, area.bottom()),
            ],
            egui::Stroke::new(1.0, ui.visuals().warn_fg_color),
        );
    }

    /// What the last edit could not do, said out loud.
    fn note(&self, ui: &Ui, painter: &egui::Painter, area: Rect) {
        let Some(text) = &self.trouble else {
            return;
        };
        painter.text(
            area.right_top() + vec2(-6.0, 4.0),
            egui::Align2::RIGHT_TOP,
            text,
            egui::FontId::proportional(11.0),
            ui.visuals().error_fg_color,
        );
    }
}

/// Every lane, gutter label and clip.
fn rows(paint: &lanes::Paint<'_>, gutter: Rect, area: Rect) {
    let top = area.top() + ruler::HEIGHT;
    for (track, offset) in lanes::laid_out(paint.project) {
        lanes::gutter(
            paint.ui,
            paint.painter,
            lanes::lane_rect(gutter, top, offset),
            track,
        );
        lanes::draw(paint, lanes::lane_rect(area, top, offset), track);
    }
}

/// How tall the panel wants to be for this project: the ruler, every lane, and
/// a little room under the last one.
///
/// Sized to the content rather than fixed, because a timeline that cuts off
/// its bottom track hides the fact that the track is there at all — and the
/// commonest projects have two or three tracks, not twenty. Clamped so a
/// project with many tracks does not swallow the preview; past the clamp the
/// panel is resizable by hand.
pub(crate) fn desired_height(project: &Project) -> f32 {
    let wanted = ruler::HEIGHT + lanes::height(project) + 12.0;
    wanted.clamp(120.0, 320.0)
}

/// The line where picture stops and sound begins.
fn divider(ui: &Ui, painter: &egui::Painter, area: Rect, project: &Project) {
    let Some(offset) = lanes::divider(project) else {
        return;
    };
    let y = area.top() + ruler::HEIGHT + offset;
    painter.line_segment(
        [egui::pos2(area.left(), y), egui::pos2(area.right(), y)],
        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
}

/// The gutter and the clip area.
fn split(full: Rect) -> (Rect, Rect) {
    let gutter = Rect::from_min_size(full.min, vec2(GUTTER, full.height()));
    let area = Rect::from_min_max(egui::pos2(full.left() + GUTTER, full.top()), full.max);
    (gutter, area)
}

/// The ruler's strip across the top of the clip area.
fn ruler_rect(area: Rect) -> Rect {
    Rect::from_min_size(area.min, vec2(area.width(), ruler::HEIGHT))
}
