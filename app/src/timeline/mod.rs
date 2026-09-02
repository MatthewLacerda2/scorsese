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
mod pacing;
mod ruler;
mod view;

use egui::{Rect, Sense, Ui, vec2};
use scorsese_core::Project;

use crate::editing::{Editing, length};
use crate::project::Open;
use crate::theme::{marks, palette};
use gesture::Gesture;
use view::View;

/// A frame count as a person reads a time. Lives with the view because that is
/// the one module allowed to turn frames into seconds; re-exported because the
/// transport under the preview reads out the same playhead this ruler labels,
/// and two spellings of a timecode would be two answers to the same question.
pub(crate) use view::timecode;

/// How wide the track-label gutter is.
///
/// Wide enough for a tag and a name beside it: `V1` and `Main` is the shape a
/// track head has, and one that truncated the name would be answering the
/// question with the half nobody chose.
const GUTTER: f32 = 124.0;
/// How much one notch of the wheel magnifies.
const ZOOM_STEP: f32 = 1.15;

/// A change to the view somebody asked for with a key.
///
/// Deferred rather than applied where it was pressed, because every one of them
/// needs a width — how far to zoom about the playhead, how much timeline has to
/// fit — and the width is not known until the panel is being laid out. The
/// alternative is the keyboard reaching into the panel's geometry, which is the
/// panel's own and changes every time the window is resized.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Look {
    /// Magnify, about the playhead.
    In,
    /// And out.
    Out,
    /// Put the whole edit in the width.
    Fit,
}

/// The timeline's own state: where it is looking, and what a hand is doing.
#[derive(Debug, Default)]
pub(crate) struct Timeline {
    view: View,
    /// A view change asked for by a key, waiting for a width to be applied to.
    asked: Option<Look>,
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

    /// Asks for a change to the view, to be applied on the next repaint.
    ///
    /// Last one wins, which is right for a key held down: three presses between
    /// two frames mean the person wants to be zoomed further in, and applying
    /// all three would make one held key travel three times as fast as the wheel.
    pub(crate) fn ask(&mut self, look: Look) {
        self.asked = Some(look);
    }

    /// Applies whatever a key asked for, now that there is a width to do it in.
    ///
    /// Zoom is anchored on the **playhead** rather than on the pointer, which
    /// is the one place this differs from the wheel — and it differs for the
    /// same reason the wheel is anchored at all. A wheel zoom is aimed: the cut
    /// you are looking at is under the pointer. A keyboard zoom is not aimed at
    /// anything, and the thing a person is looking at is where the playhead is
    /// standing.
    fn look(&mut self, area: Rect, project: &Project, playhead: scorsese_core::Frames) {
        let Some(asked) = self.asked.take() else {
            return;
        };
        match asked {
            Look::Fit => self.view.fit(length(project), area.width()),
            Look::In | Look::Out => {
                let anchor = self.view.offset_of(playhead).clamp(0.0, area.width());
                let factor = if matches!(asked, Look::In) {
                    ZOOM_STEP
                } else {
                    1.0 / ZOOM_STEP
                };
                self.view.zoom(factor, anchor);
            }
        }
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
        self.look(area, &open.project, editing.playhead);

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
        //
        // Pacing first, and exclusively: while it is running the pointer is
        // driving a factor, so the click that ends it is the end of the gesture
        // and not a click on whatever it happened to land on.
        if !self.pace(ui, open, editing) {
            self.act(ui, &response, area, hit.as_ref(), open, editing);
        }

        let whole = ui.painter_at(full);
        let fps = open.project.timeline_fps;
        whole.rect_filled(full, 0.0, palette::INK);
        whole.rect_filled(gutter, 0.0, palette::VOID);
        heading(&whole, gutter, area);

        // Two painters, and the clip between them is what makes the gutter a
        // gutter. A clip scrolled off the left of the view is drawn at a
        // negative offset; with one painter over the whole strip it slides
        // under the track heads and out the other side, which is the one thing
        // a fixed column of labels must never let happen.
        let heads = whole.with_clip_rect(gutter);
        let painter = whole.with_clip_rect(area);
        ruler::draw(&painter, ruler_rect(area), self.view, fps);
        rows(
            &lanes::Paint {
                painter: &painter,
                project: &open.project,
                view: self.view,
                selected: &editing.selected,
                highlighted: editing.highlighted.clone(),
            },
            &heads,
            gutter,
            area,
        );
        beyond(&painter, area, self.view, &open.project);
        divider(&painter, area, &open.project);
        // The gutter's own edge, drawn last of the furniture so nothing lands
        // on top of the line that says where the labels stop.
        marks::rule(
            &whole,
            gutter.right_top(),
            gutter.right_bottom(),
            palette::EDGE,
        );

        self.snap_line(&painter, area);
        self.note(&painter, area);
        ruler::playhead(
            &painter,
            area,
            area.left() + self.view.offset_of(editing.playhead),
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
    fn snap_line(&self, painter: &egui::Painter, area: Rect) {
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
        marks::rule(
            painter,
            egui::pos2(x, area.top() + ruler::HEIGHT),
            egui::pos2(x, area.bottom()),
            palette::WARM,
        );
    }

    /// What the panel has to say: the gesture in flight, and what the last
    /// edit could not do.
    ///
    /// Both in one corner and stacked, because they are two halves of one
    /// answer during a scale — the factor asked for, and the reason the clips
    /// stopped following it.
    fn note(&self, painter: &egui::Painter, area: Rect) {
        // Under the ruler and on a plate of its own. Both halves are the same
        // lesson: this was painted as bare text in the ruler's own strip, over
        // the timecodes, and a sentence you have to read *through* a row of
        // numbers is one nobody reads at all — least of all mid-gesture, which
        // is the only moment it is ever on screen.
        let mut at = egui::pos2(area.right() - 8.0, area.top() + ruler::HEIGHT + 6.0);
        for (text, colour) in self.said() {
            at.y += marks::plate(painter, at, &text, colour) + 5.0;
        }
    }

    /// What the panel has to say this frame, top line first.
    fn said(&self) -> Vec<(String, egui::Color32)> {
        let mut lines = Vec::new();
        if let Some(Gesture::Pace(pace)) = &self.gesture {
            lines.push((pace.readout(), palette::ACCENT));
        }
        if let Some(text) = &self.trouble {
            lines.push((text.clone(), palette::ALERT));
        }
        lines
    }
}

/// Every lane, track head and clip — in three passes, and the order is the
/// whole point.
///
/// Grounds first, then the grid over them, then the blocks over the grid. A
/// grid drawn under the lane grounds would be invisible; one drawn over the
/// blocks would stripe every label. Between the two it shows exactly where
/// there is no clip, which is where somebody is looking when they are judging
/// whether two cuts land on the same beat.
fn rows(paint: &lanes::Paint<'_>, heads: &egui::Painter, gutter: Rect, area: Rect) {
    let top = area.top() + ruler::HEIGHT;
    let laid = lanes::laid_out(paint.project);
    for (track, offset) in &laid {
        lanes::gutter(heads, lanes::lane_rect(gutter, top, *offset), track);
        lanes::ground(paint.painter, lanes::lane_rect(area, top, *offset));
    }
    ruler::grid(paint.painter, area, paint.view, paint.project.timeline_fps);
    for (track, offset) in &laid {
        lanes::draw(paint, lanes::lane_rect(area, top, *offset), track);
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
    let wanted = ruler::HEIGHT + lanes::height(project) + 14.0;
    wanted.clamp(120.0, 340.0)
}

/// The line where picture stops and sound begins.
///
/// Brighter than the hairline under a lane, because it divides two *kinds* of
/// row rather than two rows: video tracks composite in array order and audio
/// tracks all sum, so the halves mean different things and must not read as one
/// list.
fn divider(painter: &egui::Painter, area: Rect, project: &Project) {
    let Some(offset) = lanes::divider(project) else {
        return;
    };
    let y = area.top() + ruler::HEIGHT + offset;
    marks::rule(
        painter,
        egui::pos2(area.left(), y),
        egui::pos2(area.right(), y),
        palette::EDGE,
    );
}

/// The gutter and the clip area.
fn split(full: Rect) -> (Rect, Rect) {
    let gutter = Rect::from_min_size(full.min, vec2(GUTTER, full.height()));
    let area = Rect::from_min_max(egui::pos2(full.left() + GUTTER, full.top()), full.max);
    (gutter, area)
}

/// Where the film stops.
///
/// Everything past the last frame anything occupies is knocked back and a line
/// is drawn at the edge. Without it a timeline zoomed out looks the same a
/// second after the film ends as it does an hour after — and "how much of this
/// strip is the actual cut" is a thing you should be able to see rather than
/// work out from the ruler.
///
/// Over the lane grounds and under the clips, like the grid, for the same
/// reason: nothing this draws may dim a block somebody is looking at.
fn beyond(painter: &egui::Painter, area: Rect, view: View, project: &Project) {
    let end = area.left() + view.offset_of(length(project));
    if end >= area.right() {
        return;
    }
    let from = end.max(area.left());
    painter.rect_filled(
        Rect::from_min_max(egui::pos2(from, area.top() + ruler::HEIGHT), area.max),
        0.0,
        palette::VOID.gamma_multiply(0.55),
    );
    if end >= area.left() {
        marks::rule(
            painter,
            egui::pos2(end, area.top() + ruler::HEIGHT),
            egui::pos2(end, area.bottom()),
            palette::EDGE,
        );
    }
}

/// The panel's own name, in the corner the ruler leaves empty above the track
/// heads.
///
/// There is nowhere else for it now that the timeline has no widget heading of
/// its own, and the corner is otherwise the one piece of this window that is
/// dark and says nothing at all.
fn heading(painter: &egui::Painter, gutter: Rect, area: Rect) {
    painter.text(
        egui::pos2(gutter.left() + 10.0, area.top() + ruler::HEIGHT / 2.0),
        egui::Align2::LEFT_CENTER,
        "TIMELINE",
        egui::FontId::proportional(9.5),
        palette::ACCENT_DIM,
    );
}

/// The ruler's strip across the top of the clip area.
fn ruler_rect(area: Rect) -> Rect {
    Rect::from_min_size(area.min, vec2(area.width(), ruler::HEIGHT))
}
