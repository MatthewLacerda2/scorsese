//! The window following the document while something else edits it.
//!
//! This is the loop scorsese is built around — an assistant edits over MCP, a
//! person watches — so these drive the real `draw`, with a real project on
//! disk being rewritten underneath it, rather than calling the reload directly.

#[path = "../panels/fixture.rs"]
mod fixture;

mod following;

use egui_kittest::Harness;
use scorsese_app::Scorsese;

/// Long enough for the watch to look again. The window polls rather than
/// subscribing, so a test has to let an interval pass — 400ms against a 300ms
/// poll, which is slack enough not to be a race and short enough not to be
/// felt.
pub(crate) const SETTLE: std::time::Duration = std::time::Duration::from_millis(400);

/// A window open on `project`, drawn the way the app draws it.
pub(crate) fn window(project: &std::path::Path) -> Harness<'static, Scorsese> {
    Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, window: &mut Scorsese| window.draw(ui),
            Scorsese::opening(Some(project.to_path_buf())),
        )
}

/// Whether the document on screen contains a clip with this id.
pub(crate) fn has_clip(harness: &Harness<'static, Scorsese>, id: &str) -> bool {
    harness
        .state()
        .showing()
        .expect("a project is open")
        .clips()
        .any(|(_, clip)| clip.id.as_str() == id)
}

/// Rewrites the open document, as something outside the window would.
pub(crate) fn rewrite(project: &std::path::Path, edit: impl Fn(String) -> String) {
    let file = project.join("project.json");
    let document = std::fs::read_to_string(&file).expect("read the document");
    std::fs::write(&file, edit(document)).expect("write the document");
}
