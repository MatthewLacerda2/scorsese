//! The caption look both surfaces build a run with (#1288).

use scorsese_compositor::text::caption::{self, Choices, Stroke};
use scorsese_core::{FontChoice, ProjectPath, Rgba, TrackId};

#[test]
fn a_stroke_is_a_colour_or_none() {
    assert_eq!("none".parse(), Ok(Stroke(None)));
    assert_eq!(" None ".parse(), Ok(Stroke(None)));
    assert_eq!(
        "#ff000080".parse(),
        Ok(Stroke(Some(Rgba::new(255, 0, 0, 128))))
    );
    assert!("red".parse::<Stroke>().is_err());
}

/// Nothing chosen is the default look; each choice replaces only its part.
#[test]
fn choices_lay_over_the_default_look() {
    let track = TrackId::new("captions");
    let plain = Choices::default().captioning(track.clone(), 30.0);
    assert_eq!(plain.style, caption::style());
    assert_eq!(plain.lift, caption::LIFT);

    let chosen = Choices {
        stroke: Some(Stroke(None)),
        max_chars: Some(12),
        font: Some(FontChoice::File(ProjectPath::new("assets/face.ttf"))),
        ..Choices::default()
    }
    .captioning(track, 30.0);
    assert_eq!(chosen.style.stroke, None);
    assert_eq!(chosen.chunking.max_chars, 12);
    assert_eq!(chosen.style.weight, None, "a file is whatever weight it is");
    assert_eq!(chosen.style.color, caption::style().color);
}
