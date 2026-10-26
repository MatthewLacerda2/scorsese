//! Every variant of every enum the format writes, as it is written.
//!
//! [`every!`] takes each variant's pattern beside the value standing for it,
//! and turns the patterns into one `match` with no wildcard: a variant added to
//! the format does not compile here until it is listed, and a listed pattern
//! whose value is some other variant fails the test.

use scorsese_core::{AnchorX, AnchorY};
use scorsese_core::{
    Aspect, AssetKind, Attach, Blend, ClipId, ClipSeconds, Curve, Easing, Endpoint, Fill, Fit,
    FontChoice, GenerationState, Geometry, Heads, Linear, Point, ProjectPath, Radial, Side, Stop,
};
use serde_json::{Map, Value};

use super::document::color;

/// `(pattern => value, …)`, or just `(Unit::Variant, …)` when each variant is
/// its own value: the variants written as JSON, in the order listed.
macro_rules! every {
    ($($variant:path),+ $(,)?) => { every!($($variant => $variant),+) };
    ($($pattern:pat => $value:expr),+ $(,)?) => {{
        let values = vec![$($value),+];
        for value in &values {
            match value { $($pattern => {}),+ }
        }
        let patterns = [$(stringify!($pattern)),+];
        let mut written = Vec::new();
        for (value, pattern) in values.iter().zip(patterns) {
            let hits: Vec<bool> = vec![$(matches!(value, $pattern)),+];
            let at = patterns.iter().position(|p| *p == pattern).expect("listed");
            assert!(hits[at], "the value listed for `{pattern}` is another variant");
            written.push(serde_json::to_value(value).expect("serialise"));
        }
        Value::Array(written)
    }};
}

pub(crate) fn linear() -> Linear {
    Linear {
        angle: 90.0,
        stops: vec![
            Stop {
                color: color(),
                at: 0.0,
            },
            Stop::new(color(), 1.0),
        ],
    }
}

pub(crate) fn arrow() -> Geometry {
    Geometry::Arrow {
        from: Endpoint::At(Point { x: 0.25, y: 0.5 }),
        to: Endpoint::Attached {
            attach: Attach {
                clip: ClipId::new("c1"),
                side: Side::Top,
            },
        },
        curve: Curve::S,
        heads: Heads::Both,
    }
}

/// The table, keyed by type name.
pub(crate) fn table() -> Value {
    let mut table = Map::new();
    let mut put = |name: &str, written: Value| table.insert(name.to_owned(), written);
    put(
        "AnchorX",
        every!(AnchorX::Left, AnchorX::Center, AnchorX::Right),
    );
    put(
        "AnchorY",
        every!(AnchorY::Top, AnchorY::Center, AnchorY::Bottom),
    );
    put("Aspect", every!(Aspect::Wide, Aspect::Tall));
    put(
        "AssetKind",
        every!(
            AssetKind::Video,
            AssetKind::Image,
            AssetKind::Audio,
            AssetKind::Text,
            AssetKind::Color,
            AssetKind::Shape,
            AssetKind::Icon,
            AssetKind::ImageSequence,
            AssetKind::Html,
            AssetKind::GeneratedVideo,
            AssetKind::GeneratedImage,
            AssetKind::GeneratedAudio,
            AssetKind::SynthAudio,
            AssetKind::Group,
        ),
    );
    put(
        "Blend",
        every!(Blend::Normal, Blend::Add, Blend::Screen, Blend::Multiply),
    );
    put(
        "ClipSeconds",
        every!(ClipSeconds::Four, ClipSeconds::Six, ClipSeconds::Eight),
    );
    put("Curve", every!(Curve::Straight, Curve::S));
    put(
        "Easing",
        every!(
            Easing::Linear => Easing::Linear, Easing::EaseIn => Easing::EaseIn,
            Easing::EaseOut => Easing::EaseOut, Easing::EaseInOut => Easing::EaseInOut,
            Easing::Hold => Easing::Hold, Easing::BackIn => Easing::BackIn,
            Easing::BackOut => Easing::BackOut, Easing::BackInOut => Easing::BackInOut,
            Easing::Spring => Easing::Spring,
            Easing::CubicBezier(_) => Easing::CubicBezier([0.25, 0.0, 0.75, 1.0]),
        ),
    );
    put(
        "Endpoint",
        every!(
            Endpoint::At(_) => Endpoint::At(Point { x: 0.5, y: 0.5 }),
            Endpoint::Attached { .. } => Endpoint::Attached {
                attach: Attach { clip: ClipId::new("c1"), side: Side::Left },
            },
        ),
    );
    put(
        "Fill",
        every!(
            Fill::Solid(_) => Fill::Solid(color()),
            Fill::Linear(_) => Fill::Linear(linear()),
            Fill::Radial(_) => Fill::Radial(Radial {
                center: Point { x: 0.25, y: 0.75 }, radius: 0.5, stops: linear().stops,
            }),
        ),
    );
    put("Fit", every!(Fit::Fit, Fit::Fill, Fit::Native));
    put(
        "FontChoice",
        every!(
            FontChoice::Named(_) => FontChoice::Named("sans".to_owned()),
            FontChoice::File(_) => FontChoice::File(ProjectPath::new("fonts/face.ttf")),
        ),
    );
    put(
        "GenerationState",
        every!(
            GenerationState::Sketch,
            GenerationState::Queued,
            GenerationState::Generated,
            GenerationState::Stale,
        ),
    );
    put(
        "Geometry",
        every!(
            Geometry::Rectangle { .. } => Geometry::Rectangle { width: 0.5, height: 0.25, radius: 0.125 },
            Geometry::Ellipse { .. } => Geometry::Ellipse { width: 0.5, height: 0.25 },
            Geometry::Arrow { .. } => arrow(),
        ),
    );
    put("Heads", every!(Heads::None, Heads::End, Heads::Both));
    super::units::more(&mut put);
    Value::Object(table)
}
