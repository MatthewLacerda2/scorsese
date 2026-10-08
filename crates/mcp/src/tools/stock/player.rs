//! The page that plays a Lottie `stock_import` brought in (#903), handed back
//! in the import's reply so any client can write it — the web's assistant
//! cannot read `docs/pages.md`, whose worked page this is, recolouring aside.

/// The page that plays `file`, to write with `page_write` as it is or inside
/// a larger page: lottie-web, never autoplaying, drawing the frame for the
/// page's own time on every animation frame — so it is seekable, and the
/// same as `docs/pages.md`'s worked page, minus the recolouring.
pub(super) fn page(file: &str) -> String {
    format!(
        "Play it from an html page beside it, driven from the page's clock (docs/pages.md, \
         A Lottie animation) — this page loops it in a 720 px square; size and place the div \
         as you like, and use Math.min(at, anim.totalFrames - 1) to play it once:\n\
         <!doctype html><html><head><script src=\"https://lib.scorsese/lottie.min.js\"></script>\n\
         <style>html, body {{ margin: 0; height: 100%; }} body {{ display: grid; \
         place-content: center; }} #anim {{ width: 720px; height: 720px; }}</style></head>\n\
         <body><div id=\"anim\"></div><script>\n\
         const start = performance.now();\n\
         fetch(\"{file}\").then((r) => r.json()).then((data) => {{\n\
         \x20 const anim = lottie.loadAnimation({{ container: document.getElementById(\"anim\"),\n\
         \x20   renderer: \"svg\", loop: false, autoplay: false, animationData: data }});\n\
         \x20 const draw = (now) => {{\n\
         \x20   const at = (now - start) / 1000 * anim.frameRate;\n\
         \x20   anim.goToAndStop(at % anim.totalFrames, true);\n\
         \x20   requestAnimationFrame(draw);\n\
         \x20 }};\n\
         \x20 requestAnimationFrame(draw);\n\
         }});\n\
         </script></body></html>"
    )
}

/// The page, captured: it must draw the animation and say nothing.
#[cfg(test)]
mod tests {
    use scorsese_core::{
        Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Project, ProjectPath, Track, TrackId,
        TrackKind,
    };
    use scorsese_render::{RenderSettings, Renderer, Resolution, Tools};

    /// The hand-made stand-in `scorsese-render`'s tests play.
    const WAVE: &str = include_str!("../../../../render/tests/fixtures/lottie/wave.json");

    #[test]
    fn the_page_handed_back_plays_the_file_cleanly() {
        let dir = std::env::temp_dir().join(format!("scorsese-mcp-player-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pages")).unwrap();
        std::fs::write(dir.join("pages/lottie-1.json"), WAVE).unwrap();
        std::fs::write(dir.join("pages/player.html"), super::page("lottie-1.json")).unwrap();
        let mut project = Project::new("player", Fps::THIRTY);
        project.assets.push(Asset::imported(
            AssetId::new("player"),
            AssetKind::Html,
            ProjectPath::new("pages/player.html"),
        ));
        let mut track = Track::new(TrackId::new("v1"), TrackKind::Video);
        track.clips.push(Clip::new(
            ClipId::new("c1"),
            AssetId::new("player"),
            Frames(0),
            Frames(60),
        ));
        project.tracks.push(track);

        let tools = Tools::discover().unwrap();
        let settings = RenderSettings::new(Resolution::new(160, 90).unwrap(), Fps::THIRTY);
        let (frame, notes) = Renderer::new(&tools, settings)
            .still_noted(&project, &dir, Frames(30))
            .unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        let bytes = frame.bytes();
        assert!(
            bytes.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "the animation is drawn"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
