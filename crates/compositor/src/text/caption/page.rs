//! The page look (#1550): the look of captions on 9:16 social video. White
//! heavy upper-case type rimmed in black, a few words at a time, the word
//! being said lit in yellow and the light moving word by word with the voice.
//!
//! A page rather than native text, since native text is frozen and a new
//! graphics capability is a page: this one reads `scorsese.words`, so it
//! follows the narration's own timings, generated or transcribed.

use scorsese_core::captions::{Captioning, Chunk};
use scorsese_core::{FontChoice, Rgba, TextStyle};

/// The word being said: a warm yellow that reads against white and any shot.
pub const HIGHLIGHT: Rgba = Rgba::opaque(0xff, 0xd4, 0x00);

/// The page, its parts left as `{{NAME}}`.
const TEMPLATE: &str = include_str!("page.html");

/// The page look's type: Montserrat 900, white, a black rim about 7 px wide
/// at 1080 × 1920, sized so three upper-case words fit across a phone.
pub fn style() -> TextStyle {
    TextStyle {
        font: FontChoice::Named("montserrat".to_owned()),
        weight: Some(900),
        size: 0.038,
        color: Rgba::WHITE,
        stroke: Some(Rgba::BLACK),
        stroke_width: 0.0036,
        ..TextStyle::default()
    }
}

/// The page `chunks` are played by, set in `asked`'s style and lift: what
/// [`scorsese_core::captions::Look::Page`] is handed.
pub fn html(asked: &Captioning, chunks: &[Chunk]) -> String {
    let style = &asked.style;
    let (face, family) = match &style.font {
        FontChoice::File(path) => (
            format!(
                "@font-face {{ font-family: \"caption face\"; src: url(\"../{}\"); }}",
                escaped(path.as_str())
            ),
            "\"caption face\"".to_owned(),
        ),
        FontChoice::Named(name) => (String::new(), css_string(name)),
    };
    let weight = style
        .weight
        .map_or_else(|| "normal".to_owned(), |w| w.to_string());
    // The rim is outside the letter; a CSS stroke straddles it, so twice it,
    // with the fill painted over the inner half.
    let (stroke, width) = match style.stroke {
        Some(colour) => (colour.to_string(), number(style.stroke_width * 200.0)),
        None => ("transparent".to_owned(), "0".to_owned()),
    };
    let captions = list(chunks.iter().map(|chunk| {
        list(chunk.words.iter().map(|word| {
            let name = format!("{}/{}", chunk.line, word.name);
            list([quoted(&name), quoted(&word.text)].into_iter())
        }))
    }));
    let chunking = asked.chunking;
    [
        ("FACE", face),
        ("FAMILY", family),
        ("WEIGHT", weight),
        ("SIZE", number(style.size * 100.0)),
        ("LIFT", number(asked.lift * 100.0)),
        ("COLOR", style.color.to_string()),
        ("STROKE_WIDTH", width),
        ("STROKE", stroke),
        ("HIGHLIGHT", HIGHLIGHT.to_string()),
        ("LEAD", number(chunking.lead)),
        ("HOLD", number(chunking.hold)),
        ("BRIDGE", number(chunking.bridge)),
        ("CAPTIONS", captions),
    ]
    .iter()
    .fold(TEMPLATE.to_owned(), |page, (name, value)| {
        page.replace(&format!("{{{{{name}}}}}"), value)
    })
}

/// `x` as CSS and JavaScript both read it, without float noise.
fn number(x: f64) -> String {
    let rounded = (x * 10_000.0).round() / 10_000.0;
    format!("{rounded}")
}

/// A JavaScript array of `items`, already written.
fn list(items: impl Iterator<Item = String>) -> String {
    format!("[{}]", items.collect::<Vec<_>>().join(","))
}

/// `text` as a JavaScript string. Nothing a word says can close the script:
/// `<` is written escaped, as are quotes, backslashes and control characters.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            c if u32::from(c) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A face's name as a quoted CSS string that cannot leave the stylesheet.
fn css_string(name: &str) -> String {
    let inner: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .collect();
    format!("\"{inner}\"")
}

/// A project path as a URL path: every byte but the unreserved ones and `/`
/// percent-escaped, as a font file's `[wght]` needs.
fn escaped(path: &str) -> String {
    path.bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests;
