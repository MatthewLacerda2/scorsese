//! Making a caption, a title, a lower third: the one asset an agent writes
//! most.

use scorsese_core::{Fill, Inline, TextStyle};
use serde_json::Value;

use super::super::{color, maybe, weight};
use super::{Arguments, motion};
use crate::tools::authoring::fill::fill;

/// What the caption says, when it is missing.
const WHAT: &str = "what the caption says";

/// The text asset the arguments describe.
pub(super) fn made(arguments: &Arguments) -> Result<Inline, String> {
    // Kept as written, newlines and all — only a caption with nothing in it
    // is refused, the way a missing one is.
    let text = arguments
        .text
        .clone()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("`text` is required: {WHAT}"))?;
    let style = motion::apply(
        made_block(arguments.reveal.as_ref()),
        made_block(arguments.number.as_ref()),
        style(arguments)?,
    )?;
    Ok(Inline::Text { text, style })
}

/// A block as making reads it: `false` — which removes one from a caption
/// that has it — is simply none on a caption that is new.
fn made_block(given: Option<&Value>) -> Option<&Value> {
    given.filter(|value| **value != Value::Bool(false))
}

/// The look a new caption is set in, or `None` when nothing about it was said
/// — which leaves the style out of the document rather than writing the
/// defaults into it.
fn style(arguments: &Arguments) -> Result<Option<TextStyle>, String> {
    let mut style = TextStyle::default();
    let mut said = false;
    if let Some(font) = maybe(arguments.font.as_deref()) {
        style.font = font.into();
        said = true;
    }
    if let Some(weight) = weight(arguments.weight)? {
        style.weight = Some(weight);
        said = true;
    }
    if let Some(italic) = arguments.italic {
        style.italic = italic;
        said = true;
    }
    for (value, field) in [
        (arguments.size, &mut style.size),
        (arguments.line_height, &mut style.line_height),
        (arguments.max_width, &mut style.max_width),
        (arguments.stroke_width, &mut style.stroke_width),
    ] {
        if let Some(value) = value {
            *field = value;
            said = true;
        }
    }
    if let Some(color) = one_colour(arguments)? {
        style.color = color;
        said = true;
    }
    if let Some(align) = arguments.align {
        style.align = align.into();
        said = true;
    }
    if let Some(stroke) = color(arguments.stroke.as_deref(), "stroke")? {
        style.stroke = Some(stroke);
        said = true;
    }
    Ok(said.then_some(style))
}

/// The caption's colour: one colour, because letters are not painted with a
/// gradient here, and a gradient asked for is refused rather than flattened.
fn one_colour(arguments: &Arguments) -> Result<Option<scorsese_core::Rgba>, String> {
    match fill(arguments.color.as_ref(), "color")? {
        None => Ok(None),
        Some(Fill::Solid(color)) => Ok(Some(color)),
        Some(_) => Err(
            "`color`: a text asset is one colour, as `#rrggbb` — only a \
                        color asset takes a gradient"
                .to_owned(),
        ),
    }
}
