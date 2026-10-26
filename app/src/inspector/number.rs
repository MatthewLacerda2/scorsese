//! The web's `NumberField`: a 32px bordered box with a prefix and a suffix in
//! xs muted either side of the number, which is dragged or clicked to type.

use egui::{DragValue, Frame, Margin, Response, Stroke, Ui, emath::Numeric};

use crate::theme::{ROUND_MD, marks, palette};

/// The label column of a row of fields: the web's `5.5rem`.
pub(super) const LABELS: f32 = 88.0;

/// How tall the box is: `h-8`.
const HEIGHT: f32 = 32.0;

/// A number in the web's field, `prefix` and `suffix` set small and muted
/// around it. `drag` gives the number its speed and range.
///
/// The box takes the `ring` border while the number is focused or dragged, as
/// the web's `focus-within` field does.
pub(super) fn field<N: Numeric>(
    ui: &mut Ui,
    value: &mut N,
    prefix: &str,
    suffix: &str,
    drag: impl FnOnce(DragValue<'_>) -> DragValue<'_>,
) -> Response {
    let colours = palette::of(ui.ctx());
    let outcome = Frame::NONE
        .stroke(Stroke::new(1.0, colours.input))
        .corner_radius(ROUND_MD)
        .inner_margin(Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.set_min_height(HEIGHT - 2.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if !prefix.is_empty() {
                    ui.label(egui::RichText::new(prefix).small().weak());
                }
                // Frameless inside the box: the box is the field.
                marks::frameless(ui);
                let response = ui.add(drag(DragValue::new(value)));
                if !suffix.is_empty() {
                    ui.label(egui::RichText::new(suffix).small().weak());
                }
                response
            })
            .inner
        });
    let response = outcome.inner;
    if response.has_focus() || response.dragged() {
        ui.painter().rect_stroke(
            outcome.response.rect,
            ROUND_MD,
            Stroke::new(1.0, colours.ring),
            egui::StrokeKind::Inside,
        );
    }
    response
}
