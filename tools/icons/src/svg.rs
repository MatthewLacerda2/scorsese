//! One vendored `.svg`, read into the two things an icon draws.
//!
//! **This is not an SVG reader and must never become one.** It reads the seven
//! elements Lucide draws with, from files whose header it checks is the one
//! every Lucide icon has, and it refuses anything else out loud. That refusal
//! is the point: a `g`, a `transform` or a gradient arriving in a future
//! version has to stop a vendoring run, not be skipped into an icon that draws
//! three quarters of itself.
//!
//! Two buckets come out, because Lucide draws in two ways and only two. Almost
//! everything is stroked in `currentColor` at width 2; nineteen elements in the
//! set are dots — a `circle` of radius half a unit with `fill="currentColor"` —
//! and stroking those would draw a ring where a full stop belongs.

use crate::geometry::{Command, ellipse, rectangle};
use crate::path;

/// What one icon draws: the contours to stroke, and the contours to fill.
#[derive(Debug, Default)]
pub struct Drawing {
    /// Stroked in the caller's colour, at the caller's width.
    pub stroked: Vec<Command>,
    /// Filled in the same colour. Empty for all but a handful of icons.
    pub filled: Vec<Command>,
}

/// The header every Lucide icon carries. Checked rather than assumed: the
/// conventions are what make one stroke colour and one width the whole visual
/// vocabulary, and an icon that opted out of them would draw wrong.
const HEADER: &[(&str, &str)] = &[
    ("viewBox", "0 0 24 24"),
    ("fill", "none"),
    ("stroke", "currentColor"),
    ("stroke-width", "2"),
    ("stroke-linecap", "round"),
    ("stroke-linejoin", "round"),
];

/// Reads one icon file.
pub fn parse(source: &str) -> Result<Drawing, String> {
    let mut drawing = Drawing::default();
    let mut rest = source;
    let mut root = true;

    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        if let Some(after) = rest.strip_prefix("!--") {
            let end = after.find("-->").ok_or("an unterminated comment")?;
            rest = &after[end + 3..];
            continue;
        }
        let end = rest.find('>').ok_or("an unterminated element")?;
        let (element, after) = rest.split_at(end);
        rest = &after[1..];
        let element = element.trim_end_matches('/').trim_end();
        let (name, attributes) = split(element)?;
        if root {
            if name != "svg" {
                return Err(format!("the root element is '{name}', not 'svg'"));
            }
            check_header(&attributes)?;
            root = false;
            continue;
        }
        if name == "/svg" {
            continue;
        }
        let commands = draw(name, &attributes)?;
        if filled(&attributes)? {
            drawing.filled.extend(commands);
        } else {
            drawing.stroked.extend(commands);
        }
    }
    if drawing.stroked.is_empty() && drawing.filled.is_empty() {
        return Err("the icon draws nothing".to_string());
    }
    Ok(drawing)
}

/// The commands one element draws, in the 24-unit space.
fn draw(name: &str, attributes: &[(&str, &str)]) -> Result<Vec<Command>, String> {
    match name {
        "path" => path::parse(attribute(attributes, "d")?),
        "circle" => {
            let radius = number(attributes, "r")?;
            Ok(ellipse(centre(attributes)?, (radius, radius)))
        }
        "ellipse" => Ok(ellipse(
            centre(attributes)?,
            (number(attributes, "rx")?, number(attributes, "ry")?),
        )),
        "rect" => {
            // SVG's rule, not an invention: a rectangle given one radius is
            // rounded equally on both axes.
            let rx = optional(attributes, "rx")?;
            let ry = optional(attributes, "ry")?;
            Ok(rectangle(
                (number(attributes, "x")?, number(attributes, "y")?),
                (number(attributes, "width")?, number(attributes, "height")?),
                (rx.or(ry).unwrap_or(0.0), ry.or(rx).unwrap_or(0.0)),
            ))
        }
        "line" => Ok(vec![
            Command::Move((number(attributes, "x1")?, number(attributes, "y1")?)),
            Command::Line((number(attributes, "x2")?, number(attributes, "y2")?)),
        ]),
        "polyline" => points(attributes, false),
        "polygon" => points(attributes, true),
        other => Err(format!("'{other}' is not one of the elements Lucide draws")),
    }
}

/// A `points` list: a move, then a line to each pair after it.
fn points(attributes: &[(&str, &str)], close: bool) -> Result<Vec<Command>, String> {
    let text = attribute(attributes, "points")?;
    let numbers: Result<Vec<f64>, String> = text
        .split([' ', ',', '\n', '\t'])
        .filter(|piece| !piece.is_empty())
        .map(|piece| {
            piece
                .parse()
                .map_err(|_| format!("'{piece}' in a points list is not a number"))
        })
        .collect();
    let numbers = numbers?;
    if numbers.len() < 4 || numbers.len() % 2 != 0 {
        return Err(format!("a points list of {} numbers", numbers.len()));
    }
    let mut commands: Vec<Command> = numbers
        .chunks_exact(2)
        .enumerate()
        .map(|(index, pair)| {
            let point = (pair[0], pair[1]);
            if index == 0 {
                Command::Move(point)
            } else {
                Command::Line(point)
            }
        })
        .collect();
    if close {
        commands.push(Command::Close);
    }
    Ok(commands)
}

/// Whether this element is one of the filled dots.
fn filled(attributes: &[(&str, &str)]) -> Result<bool, String> {
    match attributes.iter().find(|(name, _)| *name == "fill") {
        None | Some((_, "none")) => Ok(false),
        Some((_, "currentColor")) => Ok(true),
        Some((_, other)) => Err(format!("fill='{other}': an icon has one colour")),
    }
}

fn check_header(attributes: &[(&str, &str)]) -> Result<(), String> {
    for (name, expected) in HEADER {
        let found = attribute(attributes, name)?;
        if found != *expected {
            return Err(format!("{name}='{found}', expected '{expected}'"));
        }
    }
    Ok(())
}

/// An element's attributes, in the order they were written: each a name and
/// the text inside its quotes.
type Attributes<'a> = Vec<(&'a str, &'a str)>;

/// An element's name and its attributes, from the text inside its angles.
fn split(element: &str) -> Result<(&str, Attributes<'_>), String> {
    let element = element.trim();
    let end = element.find(char::is_whitespace).unwrap_or(element.len());
    let (name, mut rest) = element.split_at(end);
    let mut attributes = Vec::new();
    while let Some(equals) = rest.find('=') {
        let key = rest[..equals].trim();
        let after = rest[equals + 1..]
            .strip_prefix('"')
            .ok_or_else(|| format!("attribute '{key}' has an unquoted value"))?;
        let close = after
            .find('"')
            .ok_or_else(|| format!("attribute '{key}' has an unterminated value"))?;
        attributes.push((key, &after[..close]));
        rest = &after[close + 1..];
    }
    Ok((name, attributes))
}

fn attribute<'a>(attributes: &[(&'a str, &'a str)], name: &str) -> Result<&'a str, String> {
    attributes
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| format!("no '{name}' attribute"))
}

/// The `cx`/`cy` pair a circle and an ellipse are both placed by.
fn centre(attributes: &[(&str, &str)]) -> Result<(f64, f64), String> {
    Ok((number(attributes, "cx")?, number(attributes, "cy")?))
}

fn number(attributes: &[(&str, &str)], name: &str) -> Result<f64, String> {
    let text = attribute(attributes, name)?;
    text.parse()
        .map_err(|_| format!("{name}='{text}' is not a number"))
}

fn optional(attributes: &[(&str, &str)], name: &str) -> Result<Option<f64>, String> {
    match attributes.iter().any(|(key, _)| *key == name) {
        true => number(attributes, name).map(Some),
        false => Ok(None),
    }
}
