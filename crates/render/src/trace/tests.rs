use super::{Detail, Tracing, trace};

const WHITE: [u8; 4] = [255, 255, 255, 255];
const INK: [u8; 4] = [20, 20, 24, 255];
const RED: [u8; 4] = [200, 40, 30, 255];
const SIDE: usize = 96;

/// A red disc in a thick dark ring on white — the shape of flat art: colour
/// inside a pen line, on a plain background.
fn badge(speck: bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(SIDE * SIDE * 4);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let (dx, dy) = (x as f64 - 48.0, y as f64 - 48.0);
            let r = (dx * dx + dy * dy).sqrt();
            let pixel = match r {
                r if r < 24.0 => RED,
                r if r < 32.0 => INK,
                _ => WHITE,
            };
            rgba.extend_from_slice(&pixel);
        }
    }
    if speck {
        rgba[(5 * SIDE + 5) * 4..(5 * SIDE + 5) * 4 + 4].copy_from_slice(&[30, 60, 220, 255]);
    }
    rgba
}

fn traced(tracing: Tracing) -> super::Traced {
    trace(&badge(false), SIDE, SIDE, tracing, "badge")
}

#[test]
fn a_ring_is_two_pen_strokes_and_the_colour_one_shape() {
    let traced = traced(Tracing::default());
    assert_eq!((traced.strokes, traced.shapes), (2, 1), "{}", traced.svg);
    // Most used first: the disc covers more than its ring.
    assert_eq!(traced.colours, ["#c8281e", "#141418"]);
    assert_eq!(traced.svg.matches("<clipPath ").count(), 2);
    assert!(traced.svg.contains("viewBox=\"0 0 96 96\""));
}

#[test]
fn the_lines_are_drawn_before_the_colours() {
    let svg = traced(Tracing::default()).svg;
    let ink = svg.find("class=\"ink\"").expect("a pen stroke");
    let colour = svg.find("class=\"colour\"").expect("a colour");
    assert!(ink < colour, "{svg}");
    // The marks are one group, after the clips they use.
    assert!(svg.find("</defs>") < svg.find("<g class=\"drawing\" id=\"badge\">"));
}

#[test]
fn the_background_is_dropped_unless_it_is_kept() {
    let dropped = traced(Tracing::default());
    assert!(!dropped.svg.contains("#ffffff"), "{}", dropped.svg);
    let kept = traced(Tracing {
        keep_background: true,
        ..Tracing::default()
    });
    assert_eq!(kept.shapes, 2);
    assert!(kept.svg.contains("fill=\"#ffffff\""));
}

#[test]
fn a_speck_joins_what_it_sits_on() {
    let traced = trace(&badge(true), SIDE, SIDE, Tracing::default(), "badge");
    assert_eq!(traced.colours.len(), 2, "{:?}", traced.colours);
    assert_eq!((traced.strokes, traced.shapes), (2, 1));
}

#[test]
fn noise_does_not_become_colours() {
    let mut rgba = badge(false);
    for (at, channel) in rgba.iter_mut().enumerate() {
        if at % 4 != 3 {
            *channel = channel.saturating_add((at % 7) as u8).saturating_sub(3);
        }
    }
    let traced = trace(&rgba, SIDE, SIDE, Tracing::default(), "badge");
    assert_eq!(traced.colours.len(), 2, "{:?}", traced.colours);
}

#[test]
fn transparent_pixels_are_never_traced() {
    let clear = vec![0u8; SIDE * SIDE * 4];
    let traced = trace(&clear, SIDE, SIDE, Tracing::default(), "nothing");
    assert_eq!((traced.strokes, traced.shapes), (0, 0));
    assert!(traced.colours.is_empty());
    assert!(
        traced
            .svg
            .contains("<g class=\"drawing\" id=\"nothing\">\n</g>")
    );
}

#[test]
fn the_same_picture_traces_the_same_file() {
    let tracing = Tracing {
        detail: Detail::High,
        ..Tracing::default()
    };
    assert_eq!(traced(tracing).svg, traced(tracing).svg);
}

#[test]
fn fewer_colours_than_the_picture_has_merges_them() {
    let traced = traced(Tracing {
        colours: 2,
        keep_background: true,
        ..Tracing::default()
    });
    assert!(traced.colours.len() <= 2, "{:?}", traced.colours);
}

#[test]
fn detail_is_named_in_plain_words() {
    assert_eq!(Detail::named("low"), Some(Detail::Low));
    assert_eq!(Detail::named("medium"), Some(Detail::Medium));
    assert_eq!(Detail::named("high"), Some(Detail::High));
    assert_eq!(Detail::named("ultra"), None);
}
