//! The committed icon is a picture a window can actually be given.
//!
//! `main.rs` decodes `assets/icon.png` at startup and, on failure, opens
//! without an icon rather than refusing to open — which is the right thing for
//! a person launching the app and the wrong thing to find out about in a
//! release. A missing icon looks identical to one nobody ever wired up, so the
//! asset is checked here instead, where a bad one is loud.
//!
//! What this cannot check is whether the picture is any *good*. Whether it
//! reads at the size a taskbar draws is a question for eyes; `assets/README.md`
//! carries the reasoning and the command that produced it.

/// Comfortably above every size a bar, a switcher or a dock asks for, so the
/// platform is always scaling down rather than up.
const SIZE: u32 = 512;

#[test]
fn the_window_icon_is_a_square_png_of_the_size_main_expects() {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("assets/icon.png decodes as a PNG");

    assert_eq!(
        (icon.width, icon.height),
        (SIZE, SIZE),
        "a window icon has to be square: a platform asked to scale a {}x{} \
         picture into a square one either stretches it or crops it, and which \
         of those happens is not ours to choose",
        icon.width,
        icon.height
    );
    assert_eq!(
        icon.rgba.len(),
        (SIZE * SIZE * 4) as usize,
        "four bytes a pixel, opaque included"
    );
    // The artwork is white on black and fills the frame. An icon that decoded
    // to one flat colour would pass every check above and show as a blank
    // square, which is the one failure a size assertion cannot see.
    let brightest = icon.rgba.chunks_exact(4).map(|pixel| pixel[0]).max();
    assert_eq!(brightest, Some(255), "the icon has nothing drawn on it");
}
