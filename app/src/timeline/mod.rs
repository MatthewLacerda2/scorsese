//! The timeline: tracks and clips laid out along time, with the playhead.
//!
//! Drawn rather than assembled from widgets, because a timeline is a picture
//! of time and nothing in a widget library is shaped like one. What that buys
//! is that every position on screen comes from one transform ([`view::View`]),
//! so the ruler, the clips and the playhead cannot disagree about where a
//! frame is.

mod lanes;
mod ruler;
mod view;

use egui::{Rect, Sense, Ui, vec2};
use scorsese_core::{ClipId, Project};

use crate::editing::{Editing, length};
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

/// The timeline's own state: where it is looking.
#[derive(Debug, Default)]
pub(crate) struct Timeline {
    view: View,
    /// Whether the view has been fitted to the project that is open. Fitting
    /// once on open and never again — a view that re-fits itself would undo
    /// every zoom the moment anything changed.
    fitted: bool,
}

impl Timeline {
    /// Forgets the fit, so the next project shown gets its own.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Draws the timeline and handles what happens on it.
    pub(crate) fn show(&mut self, ui: &mut Ui, project: &Project, editing: &mut Editing) {
        let full = ui.available_rect_before_wrap();
        let (gutter, area) = split(full);
        if !self.fitted {
            self.view.fit(length(project), area.width());
            self.fitted = true;
        }

        let response = ui.allocate_rect(full, Sense::click_and_drag());
        let pointer = response.hover_pos();
        self.navigate(ui, area, pointer);

        let painter = ui.painter_at(full);
        ruler::draw(
            ui,
            &painter,
            ruler_rect(area),
            self.view,
            project.timeline_fps,
        );

        let paint = lanes::Paint {
            ui,
            painter: &painter,
            project,
            view: self.view,
            selected: editing.selected.clone(),
            highlighted: editing.highlighted.clone(),
            pointer,
        };
        divider(ui, &painter, area, project);
        let hovered = self.rows(&paint, gutter, area);
        self.seek_or_select(&response, area, hovered, editing, project);

        ruler::playhead(
            &painter,
            area,
            area.left() + self.view.offset_of(editing.playhead),
            ui.visuals().error_fg_color,
        );
    }

    /// Every lane, gutter label and clip.
    fn rows(&self, paint: &lanes::Paint<'_>, gutter: Rect, area: Rect) -> Option<ClipId> {
        let top = area.top() + ruler::HEIGHT;
        let mut hovered = None;
        for (track, offset) in lanes::laid_out(paint.project) {
            let lane = |rect: Rect| {
                Rect::from_min_size(
                    egui::pos2(rect.left(), top + offset),
                    vec2(rect.width(), lanes::LANE),
                )
            };
            lanes::gutter(paint.ui, paint.painter, lane(gutter), track);
            hovered = lanes::draw(paint, lane(area), track).or(hovered);
        }
        hovered
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

    /// What a click or drag on the timeline means.
    ///
    /// On the ruler, or dragging anywhere: move the playhead. On a clip:
    /// select it. Dragging the playhead from anywhere is deliberate — scrubbing
    /// is the thing you do most, and making it need a thin strip at the top
    /// would be making the common case the fiddly one.
    fn seek_or_select(
        &self,
        response: &egui::Response,
        area: Rect,
        hovered: Option<ClipId>,
        editing: &mut Editing,
        project: &Project,
    ) {
        let Some(at) = response.interact_pointer_pos() else {
            return;
        };
        let on_ruler = at.y <= area.top() + ruler::HEIGHT;
        if response.dragged() || (response.clicked() && (on_ruler || hovered.is_none())) {
            let frame = self.view.frame_at(at.x - area.left());
            editing.playhead = frame.min(length(project));
        }
        if response.clicked()
            && !on_ruler
            && let Some(clip) = hovered
        {
            editing.selected = Some(clip);
        }
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
