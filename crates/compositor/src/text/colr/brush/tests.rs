//! What a colour resolves to, and what happens when it cannot.
//!
//! A file of its own because the module it tests is at the size gate's limit.
//! Two palettes are used: a made-up pair of entries, so that a wrong index or
//! a replaced alpha is visible as a number rather than as a shade; and the
//! emoji face's real `CPAL`, read against its own table so that the claim is
//! *where the entries come from* rather than what the face happens to be
//! painted in.

use crate::text::font::Font;

use super::*;

/// Two entries, the second half transparent so an alpha that *replaced*
/// rather than multiplied would be visible.
const ENTRIES: [Rgba; 2] = [
    Rgba {
        r: 10,
        g: 20,
        b: 30,
        a: 255,
    },
    Rgba {
        r: 40,
        g: 50,
        b: 60,
        a: 128,
    },
];

/// A colour nothing in `ENTRIES` could be mistaken for.
const CAPTION: Rgba = Rgba {
    r: 200,
    g: 100,
    b: 1,
    a: 255,
};

fn resolved() -> Palette<'static> {
    Palette {
        entries: &ENTRIES,
        foreground: CAPTION,
    }
}

/// One resolved colour as the four bytes it stands for.
fn bytes(colour: Color) -> (u8, u8, u8, u8) {
    let eight = colour.to_color_u8();
    (eight.red(), eight.green(), eight.blue(), eight.alpha())
}

#[test]
fn the_palette_is_palette_zero_of_cpal_entry_for_entry() {
    // Read against the table rather than against a remembered colour, so
    // this says *where the entries come from* rather than what the emoji
    // face happens to be painted in.
    let bytes = include_bytes!("../../../../fonts/Noto-COLRv1.ttf");
    let font = FontRef::new(bytes).expect("the emoji face compiled into this binary parses");
    let read = palette(&font);
    let cpal = font.cpal().expect("the emoji face has a CPAL");
    assert_eq!(read.len(), usize::from(cpal.num_palette_entries()));
    assert!(read.len() > 1, "and there are thousands of them");
    let records = cpal
        .color_records_array()
        .expect("a CPAL with entries has records")
        .expect("which parse");
    for (entry, record) in read.iter().zip(records.iter()).take(64) {
        assert_eq!(
            (entry.r, entry.g, entry.b, entry.a),
            (record.red, record.green, record.blue, record.alpha)
        );
    }
}

#[test]
fn a_face_with_no_cpal_has_no_palette() {
    // All eight nameable faces, which is why a colour glyph is the only
    // thing that ever asks.
    assert!(Font::sans().at(16.0).palette().is_empty());
}

#[test]
fn an_index_resolves_to_the_entry_it_names() {
    let mut said = Vec::new();
    let first = resolved()
        .colour(0, 1.0, &mut said)
        .expect("index 0 is in the palette");
    let second = resolved()
        .colour(1, 1.0, &mut said)
        .expect("index 1 is in the palette");
    assert_eq!(bytes(first), (10, 20, 30, 255));
    assert_eq!(bytes(second), (40, 50, 60, 128));
    assert!(said.is_empty());
}

#[test]
fn the_reserved_index_is_the_caption_s_own_colour() {
    // `0xFFFF` is how a face draws a symbol meant to take the text's
    // colour, and it resolves without a palette at all.
    let mut said = Vec::new();
    let tinted = resolved()
        .colour(0xffff, 1.0, &mut said)
        .expect("the reserved index always resolves");
    assert_eq!(bytes(tinted), (CAPTION.r, CAPTION.g, CAPTION.b, CAPTION.a));
}

#[test]
fn alpha_multiplies_the_entry_s_own_rather_than_replacing_it() {
    // Entry 1 is half transparent; half of it is a quarter, not a half.
    let mut said = Vec::new();
    let half = resolved()
        .colour(1, 0.5, &mut said)
        .expect("index 1 is in the palette");
    assert_eq!(bytes(half).3, 64);
}

#[test]
fn an_index_the_palette_has_no_entry_for_is_refused_and_said_out_loud() {
    let mut said = Vec::new();
    assert!(resolved().colour(9, 1.0, &mut said).is_none());
    assert_eq!(said.len(), 1);
    assert!(said[0].wanted.contains("colour 9"), "{said:?}");
}

#[test]
fn a_stop_that_cannot_be_resolved_is_dropped_and_the_rest_keep_their_places() {
    let stops = [
        ColorStop {
            offset: 0.0,
            palette_index: 0,
            alpha: 1.0,
        },
        ColorStop {
            offset: 0.5,
            palette_index: 9,
            alpha: 1.0,
        },
        ColorStop {
            offset: 1.0,
            palette_index: 1,
            alpha: 1.0,
        },
    ];
    let mut said = Vec::new();
    let built = resolved().stops(&stops, &mut said);
    let colour = |index: u16| {
        resolved()
            .colour(index, 1.0, &mut Vec::new())
            .expect("an entry the palette has")
    };
    assert_eq!(built.len(), 2, "the middle stop names nothing");
    // Each survivor keeps **its own** offset rather than being closed up
    // into the gap the dropped one left, which would move every colour on
    // the line.
    assert_eq!(built[0], GradientStop::new(0.0, colour(0)));
    assert_eq!(built[1], GradientStop::new(1.0, colour(1)));
    assert_eq!(said.len(), 1);
}

#[test]
fn the_three_spreads_the_format_defines_are_three_different_spreads() {
    let mut said = Vec::new();
    assert_eq!(spread(Extend::Pad, &mut said), SpreadMode::Pad);
    assert_eq!(spread(Extend::Repeat, &mut said), SpreadMode::Repeat);
    assert_eq!(spread(Extend::Reflect, &mut said), SpreadMode::Reflect);
    assert!(said.is_empty(), "the format defines exactly these three");
    // Only a malformed file reaches the last arm, and it pads rather than
    // refusing — but never quietly.
    assert_eq!(spread(Extend::Unknown, &mut said), SpreadMode::Pad);
    assert_eq!(said.len(), 1, "{said:?}");
}

/// A point in the font units a brush's geometry is always given in.
fn point(x: f32, y: f32) -> skrifa::raw::types::Point<f32> {
    skrifa::raw::types::Point::new(x, y)
}

/// Two stops that resolve against `resolved`, so what a gradient test measures
/// is the geometry and never the palette.
const RAMP: [ColorStop; 2] = [
    ColorStop {
        offset: 0.0,
        palette_index: 0,
        alpha: 1.0,
    },
    ColorStop {
        offset: 1.0,
        palette_index: 1,
        alpha: 1.0,
    },
];

#[test]
fn every_gradient_the_format_has_is_one_tiny_skia_can_build() {
    // Linear, radial, and the sweep `SweepGradient` describes: all three, so
    // the claim in this module's own documentation — that the translation is
    // total in practice and `Unpaintable` is for a *malformed* file — is
    // asserted rather than asserted about. A missing arm would show up as one
    // fill quietly dropped from one emoji in ten, which is exactly the
    // failure nothing end to end can see.
    let kinds = [
        Brush::LinearGradient {
            p0: point(0.0, 0.0),
            p1: point(100.0, 0.0),
            color_stops: &RAMP,
            extend: Extend::Pad,
        },
        Brush::RadialGradient {
            c0: point(50.0, 50.0),
            r0: 0.0,
            c1: point(50.0, 50.0),
            r1: 100.0,
            color_stops: &RAMP,
            extend: Extend::Repeat,
        },
        Brush::SweepGradient {
            c0: point(50.0, 50.0),
            start_angle: 0.0,
            end_angle: 180.0,
            color_stops: &RAMP,
            extend: Extend::Reflect,
        },
    ];
    let mut said = Vec::new();
    for brush in kinds {
        assert!(
            shader(&brush, &resolved(), Transform::identity(), &mut said).is_some(),
            "{brush:?}"
        );
    }
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn a_radius_below_zero_is_clamped_rather_than_refused() {
    // A normalised colour line can put a radius below zero, which no circle
    // has — but the stops either side of it are still the gradient the
    // designer drew, so the fill is kept. Refusing here would drop a layer
    // over a number the font never meant as a measurement.
    let mut said = Vec::new();
    let built = shader(
        &Brush::RadialGradient {
            c0: point(50.0, 50.0),
            r0: -20.0,
            c1: point(50.0, 50.0),
            r1: 100.0,
            color_stops: &RAMP,
            extend: Extend::Pad,
        },
        &resolved(),
        Transform::identity(),
        &mut said,
    );
    assert!(built.is_some());
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn a_gradient_with_no_stops_left_is_refused_and_said_out_loud() {
    // Every stop names an index the palette does not have, so what reaches
    // tiny-skia is a gradient of nothing — two findings, and the second is
    // the one that says the fill was dropped rather than drawn wrong.
    let stops = [ColorStop {
        offset: 0.0,
        palette_index: 9,
        alpha: 1.0,
    }];
    let mut said = Vec::new();
    let none = shader(
        &Brush::LinearGradient {
            p0: skrifa::raw::types::Point::new(0.0, 0.0),
            p1: skrifa::raw::types::Point::new(10.0, 0.0),
            color_stops: &stops,
            extend: Extend::Pad,
        },
        &resolved(),
        Transform::identity(),
        &mut said,
    );
    assert!(none.is_none());
    assert!(
        said.iter().any(|note| note.wanted.contains("gradient")),
        "{said:?}"
    );
}

#[test]
fn a_point_keeps_its_x_as_its_x() {
    let there = at(skrifa::raw::types::Point::new(3.0, -4.0));
    assert_eq!((there.x, there.y), (3.0, -4.0));
}

#[test]
fn a_note_is_recorded_once_however_often_it_happens() {
    // A report naming the same sentence forty times is a report nobody
    // finishes, and forty is what a line of emoji would produce.
    let mut said = Vec::new();
    report(&mut said, "twice".to_owned());
    report(&mut said, "twice".to_owned());
    report(&mut said, "once".to_owned());
    assert_eq!(said.len(), 2);
}
