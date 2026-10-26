//! A dialog, as `web/src/components/ui/dialog.tsx` draws one: modal over a dimmed
//! overlay, a left-aligned title (`text-base`, medium) with a description line
//! under it, a ghost close button, a `foreground/10` ring and no shadow.
//!
//! One function, so the Render and Generate dialogs cannot drift apart.

use egui::{Align, Button, Color32, Context, Frame, Id, Key, Layout, Margin, Modal, RichText, Ui};

use super::{ROUND_XL, medium_large, palette};

/// The overlay: the web's `bg-black/50`.
const OVERLAY: Color32 = Color32::from_black_alpha(128);

/// The box's padding.
const MARGIN: i8 = 16;

/// Where the dialog `id` last drew its box (an [`egui::Rect`]); the panel
/// snapshots read it under this key.
fn rect_id(id: &str) -> Id {
    Id::new(("dialog rect", id))
}

/// Draws the dialog and returns true when the person asked to close it: the
/// close button, Escape, or a click on the overlay. A dialog that is not
/// `closable` (one that reports work in flight) has none of the three.
pub(crate) fn show(
    ctx: &Context,
    id: &str,
    closable: bool,
    title: &str,
    description: &str,
    body: impl FnOnce(&mut Ui),
) -> bool {
    let colours = palette::of(ctx);
    let frame = Frame::new()
        .fill(colours.card)
        .stroke(ctx.global_style().visuals.window_stroke)
        .corner_radius(ROUND_XL)
        .inner_margin(Margin::same(MARGIN));
    let mut close = false;
    let response = Modal::new(Id::new(("dialog", id)))
        .backdrop_color(OVERLAY)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).text_style(medium_large()));
                    ui.label(RichText::new(description).weak());
                });
                if closable {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        close = ui
                            .add(Button::new("×").frame(false))
                            .on_hover_text("Close")
                            .clicked();
                    });
                }
            });
            ui.add_space(8.0);
            body(ui);
            // Where the box ended up, for a snapshot to be cropped to: the
            // modal's own area is the whole window, backdrop included.
            let boxed = ui.min_rect().expand(MARGIN as f32 + 1.0);
            ui.ctx()
                .data_mut(|data| data.insert_temp(rect_id(id), boxed));
        });
    closable
        && (close || response.should_close() || ctx.input(|input| input.key_pressed(Key::Escape)))
}
